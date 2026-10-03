import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { useCheckForUpdate, useRegisterArrow, useRemoveArrow } from './arrow';

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

describe('useRegisterArrow', () => {
	it('POSTs the identity to /v0/arrow/:ns with no body -- the selector lives in the path', async () => {
		const { result } = renderHook(() => useRegisterArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@beta' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40beta', { method: 'POST' });
	});

	it('encodes a slashed or glob selector inside the one path segment', async () => {
		const { result } = renderHook(() => useRegisterArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@feat/x' }));
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1.*' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40feat%2Fx', { method: 'POST' });
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1.*', { method: 'POST' });
	});
});

describe('useRemoveArrow', () => {
	it('DELETEs /v0/arrow/:ns', async () => {
		const { result } = renderHook(() => useRemoveArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1', { method: 'DELETE' });
	});
});

describe('useCheckForUpdate', () => {
	it('PATCHes /v0/arrow/:ns with no body', async () => {
		const { result } = renderHook(() => useCheckForUpdate(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@stable' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40stable', { method: 'PATCH' });
	});
});
