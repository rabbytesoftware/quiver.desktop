import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import type { ArrowDetail, ArrowState } from '@/domain/arrow';
import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { useArrowStore } from '@/lib/core-store';
import { apiFetch } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { useSelectorSwitch } from './use-selector-switch';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

const FROM = 'github.com/char2cs/crowbar@stable';
const TO = 'github.com/char2cs/crowbar@beta';
const enc = encodeURIComponent;

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: FROM,
		name: 'crowbar',
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
		resolved_ref: 'v1.3.0',
		installed_commit: 'abc',
		available: null,
		outdated: false,
		active_run: null,
		last_return: null,
		channels: [],
		readme: null,
		dependencies: [],
		dependents: [],
		...overrides,
	};
}

function seedStore(namespace: string, state: ArrowState): void {
	useArrowStore.getState().reset();
	useArrowStore.getState().setCatalog([
		{
			connectionId: 'c',
			namespace,
			name: 'x',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '',
		},
	]);
	moveTo(namespace, state);
}

function moveTo(namespace: string, state: ArrowState): void {
	useArrowStore.getState().applyRuntimeUpdate({ namespace, state, active_run: null, last_return: null });
}

function calls(): string[] {
	return mockApiFetch.mock.calls.map(
		([path, init]) => `${(init as RequestInit | undefined)?.method ?? 'GET'} ${path}`
	);
}

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockResolvedValue(undefined);
});

afterEach(() => {
	resetBackend();
});

describe('useSelectorSwitch', () => {
	it('uninstalls, waits for the row to settle at absent, forgets it, then registers and installs the new identity', async () => {
		seedStore(FROM, 'ready');
		const onIdentityChange = vi.fn();
		const { result } = renderHook(() => useSelectorSwitch(detail(), { PORT: '1' }, onIdentityChange), { wrapper });

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('beta');
		});
		await waitFor(() => expect(calls()).toEqual([`POST /v0/runtime/${enc(FROM)}/uninstall`]));
		expect(result.current.pending).toBe(true);

		act(() => moveTo(FROM, 'uninstalling'));
		act(() => moveTo(FROM, 'absent'));
		await act(() => done);

		expect(calls()).toEqual([
			`POST /v0/runtime/${enc(FROM)}/uninstall`,
			`DELETE /v0/arrow/${enc(FROM)}`,
			`POST /v0/arrow/${enc(TO)}`,
			`POST /v0/runtime/${enc(TO)}/install`,
		]);
		const install = mockApiFetch.mock.calls[3][1] as RequestInit;
		expect(JSON.parse(String(install.body))).toEqual({ variables: { PORT: '1' } });
		expect(onIdentityChange).toHaveBeenCalledWith(TO);
		expect(result.current.pending).toBe(false);
		expect(result.current.error).toBeNull();
	});

	it('only swaps the catalog rows when nothing is on disk', async () => {
		seedStore(FROM, 'absent');
		const onIdentityChange = vi.fn();
		const { result } = renderHook(() => useSelectorSwitch(detail({ state: 'absent' }), {}, onIdentityChange), {
			wrapper,
		});

		await act(() => result.current.switchTo('v1.2.0'));

		const pinned = 'github.com/char2cs/crowbar@v1.2.0';
		expect(calls()).toEqual([`DELETE /v0/arrow/${enc(FROM)}`, `POST /v0/arrow/${enc(pinned)}`]);
		expect(onIdentityChange).toHaveBeenCalledWith(pinned);
	});

	it('does nothing for the selector the row already follows', async () => {
		const { result } = renderHook(() => useSelectorSwitch(detail(), {}, vi.fn()), { wrapper });
		await act(() => result.current.switchTo('stable'));
		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('stops, keeps the old row, and reports it when the uninstall settles anywhere but absent', async () => {
		seedStore(FROM, 'ready');
		const onIdentityChange = vi.fn();
		const { result } = renderHook(() => useSelectorSwitch(detail(), {}, onIdentityChange), { wrapper });

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('beta');
		});
		await waitFor(() => expect(mockApiFetch).toHaveBeenCalledTimes(1));
		act(() => moveTo(FROM, 'uninstalling'));
		act(() => moveTo(FROM, 'ready'));
		await act(() => done);

		expect(calls()).toEqual([`POST /v0/runtime/${enc(FROM)}/uninstall`]);
		expect(result.current.error).not.toBeNull();
		expect(onIdentityChange).not.toHaveBeenCalled();

		act(() => result.current.dismissError());
		expect(result.current.error).toBeNull();
	});

	it('reports a rejected uninstall without waiting on a state that will never come', async () => {
		seedStore(FROM, 'ready');
		mockApiFetch.mockRejectedValueOnce(new Error('state violation'));
		const { result } = renderHook(() => useSelectorSwitch(detail(), {}, vi.fn()), { wrapper });

		await act(() => result.current.switchTo('beta'));

		expect(result.current.error).toBe('state violation');
		expect(result.current.pending).toBe(false);
	});

	it('installs Quiver’s own new row with the release asset of the ref that new row resolved to', async () => {
		const from = `${QUIVER_DESKTOP_NAMESPACE}@stable`;
		const to = `${QUIVER_DESKTOP_NAMESPACE}@nightly-latest`;
		seedStore(from, 'ready');
		const resolveReleaseAsset = vi.fn().mockResolvedValue({
			tag: 'nightly-latest',
			name: 'q.AppImage',
			url: 'https://x/q.AppImage',
			checksum: 'c'.repeat(64),
		});
		installBackend({ resolveReleaseAsset } as unknown as Backend);
		mockApiFetch.mockImplementation((_path: string, init?: RequestInit) =>
			Promise.resolve(
				init?.method === undefined
					? { namespace: to, resolved_ref: 'nightly-latest', selector_kind: 'channel' }
					: undefined
			)
		);
		const { result } = renderHook(
			() => useSelectorSwitch(detail({ namespace: from, selector: 'stable' }), {}, vi.fn()),
			{ wrapper }
		);

		let done: Promise<void> = Promise.resolve();
		act(() => {
			done = result.current.switchTo('nightly-latest');
		});
		await waitFor(() => expect(mockApiFetch).toHaveBeenCalledTimes(1));
		act(() => moveTo(from, 'absent'));
		await act(() => done);

		expect(resolveReleaseAsset).toHaveBeenCalledWith('nightly-latest');
		const [, install] = mockApiFetch.mock.calls[mockApiFetch.mock.calls.length - 1];
		expect(JSON.parse(String((install as RequestInit).body))).toEqual({
			variables: { QUIVER_RELEASE_ASSET_URL: 'https://x/q.AppImage', QUIVER_RELEASE_CHECKSUM: 'c'.repeat(64) },
		});
		expect(calls()).toContain(`GET /v0/arrow/${enc(to)}`);
	});
});
