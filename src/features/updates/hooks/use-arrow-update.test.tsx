import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowDetail } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import { ApiError, apiFetch, apiRequest } from '@/lib/transport/api';

import { useArrowUpdate } from './use-arrow-update';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn(), apiRequest: vi.fn() };
});

const mockApiFetch = vi.mocked(apiFetch);
const mockApiRequest = vi.mocked(apiRequest);

const NS = 'github.com/rabbytesoftware/quiver.core@stable';
const STAGED = { version: 'stable-26.6', staged_at: '2026-10-03T10:00:00Z' };

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: NS,
		name: 'Quiver Core',
		description: '',
		license: '',
		url: '',
		tags: [],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [],
		targets: [],
		state: 'ready',
		user_installed: true,
		selector: 'stable',
		selector_kind: 'channel',
		resolved_ref: 'stable-26.5',
		installed_commit: '',
		available: null,
		outdated: false,
		active_run: null,
		last_return: null,
		pending_activation: null,
		channels: [],
		readme: null,
		dependencies: [],
		dependents: [],
		...overrides,
	};
}

let client: QueryClient;
let invalidate: ReturnType<typeof vi.spyOn>;

function wrapper({ children }: { children: ReactNode }) {
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function setup(initial: ArrowDetail | undefined) {
	return renderHook(({ d }: { d: ArrowDetail | undefined }) => useArrowUpdate(d), {
		wrapper,
		initialProps: { d: initial },
	});
}

beforeEach(() => {
	mockApiFetch.mockReset().mockResolvedValue(undefined);
	mockApiRequest.mockReset().mockResolvedValue({ status: 202, data: undefined });
	client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	invalidate = vi.spyOn(client, 'invalidateQueries');
	useArrowStore.getState().reset();
	useArrowStore.getState().setCatalog([
		{
			connectionId: 'c',
			namespace: NS,
			name: 'Quiver Core',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: 'stable-26.5',
		},
	]);
});

afterEach(() => {
	vi.useRealTimers();
});

describe('useArrowUpdate state', () => {
	it('reads installed, available and pending from the detail', () => {
		const { result } = setup(
			detail({ available: { ref: 'stable-26.6', commit: 'abc' }, pending_activation: STAGED })
		);
		expect(result.current.installed).toBe('stable-26.5');
		expect(result.current.available).toBe('stable-26.6');
		expect(result.current.pending).toEqual(STAGED);
	});

	it('is upToDate with nothing ahead, and empty before the detail loads', () => {
		expect(setup(detail()).result.current.state).toBe('upToDate');
		const loading = setup(undefined).result.current;
		expect(loading).toMatchObject({ state: 'upToDate', installed: '', available: null, pending: null });
	});

	it.each([
		['available', { available: { ref: 'stable-26.6', commit: 'abc' } }],
		['staged', { pending_activation: STAGED }],
		['downloading', { state: 'updating' as const }],
		['downloading', { active_run: { method: 'update', variables: {}, steps: [] }, state: 'ready' as const }],
	])('is %s', (state, overrides) => {
		expect(setup(detail(overrides)).result.current.state).toBe(state);
	});

	it('does not treat an unrelated run as an update in flight', () => {
		const { result } = setup(detail({ active_run: { method: 'execute', variables: {}, steps: [] } }));
		expect(result.current.state).toBe('upToDate');
	});
});

describe('useArrowUpdate check', () => {
	it('asks core to check this namespace, shows checking meanwhile, then re-reads the detail', async () => {
		let release!: () => void;
		mockApiFetch.mockImplementation(
			() => new Promise<undefined>((resolve) => (release = () => resolve(undefined)))
		);
		const { result } = setup(detail());

		let pending!: Promise<void>;
		act(() => {
			pending = result.current.check();
		});
		await waitFor(() => expect(result.current.checking).toBe(true));
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}/check`, { method: 'POST' });

		await act(async () => {
			release();
			await pending;
		});
		expect(result.current.checking).toBe(false);
		expect(invalidate).toHaveBeenCalled();
	});

	it('records a failed check and rejects', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('rate limited', 429));
		const { result } = setup(detail());
		await act(async () => {
			await expect(result.current.check()).rejects.toThrow('rate limited');
		});
		expect(result.current.state).toBe('error');
		expect(result.current.error).toEqual({ kind: 'rate_limited', message: 'rate limited' });
		expect(result.current.checking).toBe(false);
	});

	it('clears an error when dismissed', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('nope', 500));
		const { result } = setup(detail());
		await act(async () => {
			await result.current.check().catch(() => undefined);
		});
		act(() => result.current.dismissError());
		expect(result.current.state).toBe('upToDate');
	});
});

describe('useArrowUpdate update', () => {
	it('starts the update with no variables: core resolves its own release', async () => {
		const { result } = setup(detail({ available: { ref: 'stable-26.6', commit: 'abc' } }));
		await act(() => result.current.update());
		expect(mockApiRequest).toHaveBeenCalledWith(
			`/v0/runtime/${encodeURIComponent(NS)}/update`,
			expect.objectContaining({ body: JSON.stringify({ variables: {} }) })
		);
		expect(result.current.error).toBeNull();
	});

	it('is downloading from the click, before the runtime frame for the run arrives', async () => {
		let answer!: () => void;
		mockApiRequest.mockImplementation(
			() => new Promise((resolve) => (answer = () => resolve({ status: 202, data: undefined })))
		);
		const { result } = setup(detail({ available: { ref: 'stable-26.6', commit: 'abc' } }));

		let started!: Promise<void>;
		act(() => {
			started = result.current.update();
		});
		await waitFor(() => expect(result.current.state).toBe('downloading'));

		await act(async () => {
			answer();
			await started;
		});
		await waitFor(() => expect(result.current.state).toBe('available'));
	});

	it('re-reads the detail when core answers that nothing is newer', async () => {
		mockApiRequest.mockResolvedValue({ status: 200, data: undefined });
		const { result } = setup(detail({ available: { ref: 'stable-26.6', commit: 'abc' } }));
		await act(() => result.current.update());
		expect(invalidate).toHaveBeenCalled();
	});

	it('records a typed error and lets the next attempt clear it', async () => {
		mockApiRequest.mockRejectedValueOnce(new ApiError('another update is underway', 409));
		const { result } = setup(detail({ available: { ref: 'stable-26.6', commit: 'abc' } }));
		await act(async () => {
			await expect(result.current.update()).rejects.toThrow('another update is underway');
		});
		expect(result.current.state).toBe('error');
		expect(result.current.error?.kind).toBe('busy');

		await act(() => result.current.update());
		expect(result.current.error).toBeNull();
	});
});

describe('useArrowUpdate activate', () => {
	it('is restarting from the click until the new version reports with nothing pending', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		const { result, rerender } = setup(detail({ pending_activation: STAGED }));

		await act(() => result.current.activate());
		expect(mockApiRequest).toHaveBeenCalledWith(
			`/v0/runtime/${encodeURIComponent(NS)}/activate`,
			expect.objectContaining({ method: 'POST' })
		);
		expect(result.current.state).toBe('restarting');

		invalidate.mockClear();
		await act(() => vi.advanceTimersByTimeAsync(1600));
		expect(invalidate).toHaveBeenCalled();
		expect(result.current.state).toBe('restarting');

		rerender({ d: detail({ resolved_ref: 'stable-26.6', pending_activation: null }) });
		await waitFor(() => expect(result.current.state).toBe('upToDate'));
		expect(result.current.installed).toBe('stable-26.6');
		expect(refresh).toHaveBeenCalled();

		invalidate.mockClear();
		await act(() => vi.advanceTimersByTimeAsync(5000));
		expect(invalidate).not.toHaveBeenCalled();
	});

	it('treats a dropped connection as the daemon restarting, not as a failure', async () => {
		mockApiRequest.mockRejectedValue(new TypeError('Failed to fetch'));
		const { result } = setup(detail({ pending_activation: STAGED }));
		await act(() => result.current.activate());
		expect(result.current.state).toBe('restarting');
		expect(result.current.error).toBeNull();
	});

	it('stops restarting and records the error when the daemon refuses', async () => {
		mockApiRequest.mockRejectedValue(new ApiError('boom', 500));
		const { result } = setup(detail({ pending_activation: STAGED }));
		await act(async () => {
			await expect(result.current.activate()).rejects.toThrow('boom');
		});
		expect(result.current.state).toBe('error');
		expect(result.current.error).toEqual({ kind: 'failed', message: 'boom' });
	});

	it('does not stay restarting when core says nothing was pending', async () => {
		mockApiRequest.mockResolvedValue({ status: 200, data: undefined });
		const { result } = setup(detail({ pending_activation: null }));
		await act(() => result.current.activate());
		expect(result.current.state).toBe('upToDate');
		expect(invalidate).toHaveBeenCalled();
	});

	it('gives up with an offline error when the daemon never comes back', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const { result } = setup(detail({ pending_activation: STAGED }));
		await act(() => result.current.activate());
		await act(() => vi.advanceTimersByTimeAsync(61_000));
		expect(result.current.state).toBe('error');
		expect(result.current.error?.kind).toBe('offline');
	});
});

describe('useArrowUpdate live staging', () => {
	it('re-reads the detail when a runtime frame changes what is staged, including clearing it', async () => {
		setup(detail());
		invalidate.mockClear();

		act(() => {
			useArrowStore.getState().applyRuntimeUpdate({
				namespace: NS,
				state: 'ready',
				active_run: null,
				last_return: null,
				pending_activation: STAGED,
			});
		});
		await waitFor(() => expect(invalidate).toHaveBeenCalledTimes(1));

		act(() => {
			useArrowStore.getState().applyRuntimeUpdate({
				namespace: NS,
				state: 'ready',
				active_run: null,
				last_return: null,
				pending_activation: null,
			});
		});
		await waitFor(() => expect(invalidate).toHaveBeenCalledTimes(2));
	});

	it('does not re-read for a frame that leaves staging unchanged', async () => {
		setup(detail());
		invalidate.mockClear();
		act(() => {
			useArrowStore.getState().applyRuntimeUpdate({
				namespace: NS,
				state: 'running',
				active_run: null,
				last_return: null,
				pending_activation: null,
			});
		});
		await act(() => Promise.resolve());
		expect(invalidate).not.toHaveBeenCalled();
	});
});
