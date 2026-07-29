import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { useFollowCollection, useUnfollowCollection } from './collection';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

function wrapper() {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
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
		const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection/col%2Fns/follow', { method: 'POST' });
	});
});

describe('useUnfollowCollection', () => {
	it('DELETEs /v0/collection/:ns/follow', async () => {
		const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection/col%2Fns/follow', { method: 'DELETE' });
	});
});
