import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiFetch } from '@/lib/transport/api';

import {
	arrowChannelsQueryKey,
	arrowDependenciesQueryKey,
	arrowDependentsQueryKey,
	arrowDetailQueryKey,
	arrowReadmeQueryKey,
	useArrowChannels,
	useArrowDependencies,
	useArrowDependents,
	useArrowDetail,
	useArrowReadme,
} from './arrow';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});

const mockApiFetch = vi.mocked(apiFetch);

const NS = 'github.com/rabbyte/minecraft@v1.21.4';
const BARE_NS = 'github.com/rabbyte/minecraft';

const DETAIL = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	version: '1.21.4',
	description: 'A server.',
	license: 'MIT',
	state: 'ready',
	tags: [],
	installed_ref: 'v1.21.4',
	installed_at: '2026-05-09T21:26:59Z',
	user_installed: true,
};

const MANIFEST = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	description: 'A server.',
	tags: [],
	variables: [],
	targets: {},
	manifest: { url: '', maintainers: [], credits: [], media: {}, netbridge: [] },
};

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
	mockApiFetch.mockReset();
});

/**
 * `useArrowDetail` is now the FAST/primary query only -- `detail` + `manifest`,
 * confirmed live as simple, fast reads. The four slow endpoints (channels,
 * readme, dependencies, dependents) are their own hooks below, each with its
 * own loading/error state; `arrow-details-screen.tsx` merges their results
 * into the same `ArrowDetail` shape once each resolves. This is exactly the
 * split the old, single-`Promise.all` version of this hook used to bundle
 * into one query that gated the whole page on its slowest call.
 */
describe('useArrowDetail', () => {
	it('fetches only detail and manifest -- never readme/channels/dependencies/dependents', async () => {
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
			return Promise.resolve(DETAIL);
		});

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(mockApiFetch).toHaveBeenCalledTimes(2);
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}`);
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(BARE_NS)}/manifest`);
	});

	it('resolves immediately with placeholder values for the four slow fields', async () => {
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
			return Promise.resolve(DETAIL);
		});

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.channels).toEqual([]);
		expect(result.current.data?.readme).toBeNull();
		expect(result.current.data?.dependencies).toEqual([]);
		expect(result.current.data?.dependents).toEqual([]);
	});

	it('fails when manifest fails, same as before the split', async () => {
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/manifest')) return Promise.reject(new ApiError('internal error', 500));
			return Promise.resolve(DETAIL);
		});

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});
});

describe('useArrowChannels', () => {
	it('fetches from the bare namespace, like manifest and readme', async () => {
		mockApiFetch.mockResolvedValue({ channels: [] });

		renderHook(() => useArrowChannels(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(BARE_NS)}/channels`)
		);
	});

	it('resolves to the mapped channel list', async () => {
		mockApiFetch.mockResolvedValue({
			channels: [{ name: 'stable', kind: 'ordered', latest: 'v1.21.4', count: 1, members: ['v1.21.4'] }],
		});

		const { result } = renderHook(() => useArrowChannels(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([
			{ name: 'stable', kind: 'ordered', latest: 'v1.21.4', count: 1, members: ['v1.21.4'] },
		]);
	});

	it('resolves to an empty list when the endpoint 404s, without erroring', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('not found', 404));

		const { result } = renderHook(() => useArrowChannels(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([]);
	});

	it('errors for a reason other than 404', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('internal error', 500));

		const { result } = renderHook(() => useArrowChannels(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});
});

describe('useArrowReadme', () => {
	it('fetches from the bare namespace', async () => {
		mockApiFetch.mockResolvedValue({ namespace: BARE_NS, readme: '# About' });

		renderHook(() => useArrowReadme(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(BARE_NS)}/readme`)
		);
	});

	it('resolves to the fetched prose', async () => {
		mockApiFetch.mockResolvedValue({ namespace: BARE_NS, readme: '# About\n\nA server.' });

		const { result } = renderHook(() => useArrowReadme(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toBe('# About\n\nA server.');
	});

	it('resolves to null when the endpoint 404s, without erroring', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('not found', 404));

		const { result } = renderHook(() => useArrowReadme(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toBeNull();
	});

	it('errors for a reason other than 404', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('internal error', 500));

		const { result } = renderHook(() => useArrowReadme(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});
});

describe('useArrowDependencies', () => {
	it('fetches from the full namespace@ref, not the bare namespace -- the resolved plan is version-specific', async () => {
		mockApiFetch.mockResolvedValue({ namespace: BARE_NS, dependencies: [] });

		renderHook(() => useArrowDependencies(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}/dependencies`)
		);
	});

	it('resolves to the mapped dependency list', async () => {
		mockApiFetch.mockResolvedValue({
			namespace: BARE_NS,
			dependencies: [{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' }],
		});

		const { result } = renderHook(() => useArrowDependencies(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' }]);
	});

	it('resolves to an empty list when the endpoint 404s, without erroring', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('not found', 404));

		const { result } = renderHook(() => useArrowDependencies(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([]);
	});

	it('errors for a reason other than 404', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('internal error', 500));

		const { result } = renderHook(() => useArrowDependencies(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});
});

describe('useArrowDependents', () => {
	it('fetches from the full namespace@ref', async () => {
		mockApiFetch.mockResolvedValue({ namespace: BARE_NS, dependents: [] });

		renderHook(() => useArrowDependents(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}/dependents`)
		);
	});

	it('resolves to the fetched list', async () => {
		mockApiFetch.mockResolvedValue({ namespace: BARE_NS, dependents: ['github.com/rabbyte/discord@v1.2.0'] });

		const { result } = renderHook(() => useArrowDependents(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual(['github.com/rabbyte/discord@v1.2.0']);
	});

	it('resolves to an empty list when the endpoint 404s, without erroring', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('not found', 404));

		const { result } = renderHook(() => useArrowDependents(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([]);
	});

	it('errors for a reason other than 404', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('internal error', 500));

		const { result } = renderHook(() => useArrowDependents(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});
});

describe('query keys', () => {
	it('builds a stable key from the namespace for each of the five queries', () => {
		expect(arrowDetailQueryKey(NS)).toEqual(['arrow', NS]);
		expect(arrowChannelsQueryKey(NS)).toEqual(['arrow', NS, 'channels']);
		expect(arrowReadmeQueryKey(NS)).toEqual(['arrow', NS, 'readme']);
		expect(arrowDependenciesQueryKey(NS)).toEqual(['arrow', NS, 'dependencies']);
		expect(arrowDependentsQueryKey(NS)).toEqual(['arrow', NS, 'dependents']);
	});
});
