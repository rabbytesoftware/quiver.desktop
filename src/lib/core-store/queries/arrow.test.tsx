import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiFetch } from '@/lib/transport/api';

import { arrowDetailQueryKey, useArrowDetail } from './arrow';

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

function mockEndpoints(readme: unknown) {
	mockApiFetch.mockImplementation((path: string) => {
		if (path.endsWith('/readme')) {
			if (readme instanceof Error) return Promise.reject(readme);
			return Promise.resolve({ namespace: BARE_NS, readme });
		}
		if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
		if (path.endsWith('/dependencies')) return Promise.resolve({ namespace: BARE_NS, dependencies: [] });
		if (path.endsWith('/dependents')) return Promise.resolve({ namespace: BARE_NS, dependents: [] });
		return Promise.resolve(DETAIL);
	});
}

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useArrowDetail', () => {
	beforeEach(() => {
		mockApiFetch.mockReset();
	});

	it('fetches the readme and manifest from the bare namespace, not namespace@ref', async () => {
		mockEndpoints('# About\n\nA server.');

		renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(BARE_NS)}/readme`)
		);
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(BARE_NS)}/manifest`);
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}`);
	});

	it('resolves readme to the fetched prose', async () => {
		mockEndpoints('# About\n\nA server.');

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.readme).toBe('# About\n\nA server.');
	});

	it('resolves readme to null when the readme endpoint 404s, without failing the whole query', async () => {
		mockEndpoints(new ApiError('not found', 404));

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.readme).toBeNull();
	});

	it('fails the whole query when the readme endpoint fails for a reason other than 404', async () => {
		mockEndpoints(new ApiError('internal error', 500));

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});

	it('fetches dependencies and dependents from the full namespace@ref, not the bare namespace', async () => {
		mockEndpoints(null);

		renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() =>
			expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}/dependencies`)
		);
		expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}/dependents`);
	});

	it('resolves dependencies and dependents to the fetched data', async () => {
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/readme')) return Promise.reject(new ApiError('not found', 404));
			if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
			if (path.endsWith('/dependencies')) {
				return Promise.resolve({
					namespace: BARE_NS,
					dependencies: [{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' }],
				});
			}
			if (path.endsWith('/dependents')) {
				return Promise.resolve({ namespace: BARE_NS, dependents: ['github.com/rabbyte/discord@v1.2.0'] });
			}
			return Promise.resolve(DETAIL);
		});

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.dependencies).toEqual([
			{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' },
		]);
		expect(result.current.data?.dependents).toEqual(['github.com/rabbyte/discord@v1.2.0']);
	});

	it('falls back to an empty list when dependencies or dependents 404, without failing the whole query', async () => {
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/readme')) return Promise.reject(new ApiError('not found', 404));
			if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
			if (path.endsWith('/dependencies')) return Promise.reject(new ApiError('not found', 404));
			if (path.endsWith('/dependents')) return Promise.reject(new ApiError('not found', 404));
			return Promise.resolve(DETAIL);
		});

		const { result } = renderHook(() => useArrowDetail(NS), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.dependencies).toEqual([]);
		expect(result.current.data?.dependents).toEqual([]);
	});
});

describe('arrowDetailQueryKey', () => {
	it('builds a stable key from the namespace', () => {
		expect(arrowDetailQueryKey(NS)).toEqual(['arrow', NS]);
	});
});
