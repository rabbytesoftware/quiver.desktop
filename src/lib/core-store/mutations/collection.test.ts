import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { useFollowCollection, useUnfollowCollection } from './collection';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

function wrapper(qc: QueryClient) {
	function Wrapper({ children }: { children: React.ReactNode }) {
		return createElement(QueryClientProvider, { client: qc }, children);
	}
	return Wrapper;
}

beforeEach(() => {
	mockApiFetch.mockResolvedValue(undefined);
});

describe('useFollowCollection', () => {
	it('POSTs to /v0/collection/:ns/follow', async () => {
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper(new QueryClient()) });
		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection/col%2Fns/follow', { method: 'POST' });
	});

	it('optimistically flips followed to true in the collection query cache', async () => {
		const qc = new QueryClient();
		qc.setQueryData(['collection', 'col/ns'], { namespace: 'col/ns', followed: false });
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper(qc) });

		const mutation = act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		await waitFor(() => expect(qc.getQueryData(['collection', 'col/ns'])).toMatchObject({ followed: true }));
		await mutation;
	});

	it('rolls back the optimistic flip if the request fails', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('boom'));
		const qc = new QueryClient();
		qc.setQueryData(['collection', 'col/ns'], { namespace: 'col/ns', followed: false });
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper(qc) });

		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }).catch(() => {}));

		expect(qc.getQueryData(['collection', 'col/ns'])).toMatchObject({ followed: false });
	});

	it('does nothing to the cache when the collection was never fetched', async () => {
		const qc = new QueryClient();
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper(qc) });

		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));

		expect(qc.getQueryData(['collection', 'col/ns'])).toBeUndefined();
	});

	it('does not throw rolling back when there was nothing cached and the request fails', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('boom'));
		const qc = new QueryClient();
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper(qc) });

		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }).catch(() => {}));

		expect(qc.getQueryData(['collection', 'col/ns'])).toBeUndefined();
	});
});

describe('useUnfollowCollection', () => {
	it('DELETEs /v0/collection/:ns/follow', async () => {
		const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper(new QueryClient()) });
		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection/col%2Fns/follow', { method: 'DELETE' });
	});

	it('optimistically flips followed to false in the collection query cache', async () => {
		const qc = new QueryClient();
		qc.setQueryData(['collection', 'col/ns'], { namespace: 'col/ns', followed: true });
		const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper(qc) });

		const mutation = act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		await waitFor(() => expect(qc.getQueryData(['collection', 'col/ns'])).toMatchObject({ followed: false }));
		await mutation;
	});

	it('rolls back the optimistic flip if the request fails', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('boom'));
		const qc = new QueryClient();
		qc.setQueryData(['collection', 'col/ns'], { namespace: 'col/ns', followed: true });
		const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper(qc) });

		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }).catch(() => {}));

		expect(qc.getQueryData(['collection', 'col/ns'])).toMatchObject({ followed: true });
	});

	it('does not throw rolling back when there was nothing cached and the request fails', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('boom'));
		const qc = new QueryClient();
		const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper(qc) });

		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }).catch(() => {}));

		expect(qc.getQueryData(['collection', 'col/ns'])).toBeUndefined();
	});
});
