import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { useRegisterArrow, useRemoveArrow, useSwitchArrowChannel } from './arrow';

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
	it('POSTs to /v0/arrow/:ns with no body when no channel is given', async () => {
		const { result } = renderHook(() => useRegisterArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1', { method: 'POST' });
	});

	it('sends the channel as a JSON body when one is given', async () => {
		const { result } = renderHook(() => useRegisterArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1', channel: 'beta' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ channel: 'beta' }),
		});
	});
});

describe('useRemoveArrow', () => {
	it('DELETEs /v0/arrow/:ns', async () => {
		const { result } = renderHook(() => useRemoveArrow(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1', { method: 'DELETE' });
	});
});

describe('useSwitchArrowChannel', () => {
	it('PATCHes /v0/arrow/:ns with the channel and ref', async () => {
		const { result } = renderHook(() => useSwitchArrowChannel(), { wrapper: wrapper() });
		await act(() =>
			result.current.mutateAsync({ namespace: 'github.com/x/y@v1', channel: 'beta', ref: 'beta-1.3.0' })
		);
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow/github.com%2Fx%2Fy%40v1', {
			method: 'PATCH',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ channel: 'beta', ref: 'beta-1.3.0' }),
		});
	});

	it('omits ref from the body when none is given, letting core resolve the channel latest', async () => {
		const { result } = renderHook(() => useSwitchArrowChannel(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'github.com/x/y@v1', channel: 'beta' }));
		const calls = mockApiFetch.mock.calls;
		const call = calls[calls.length - 1];
		expect(JSON.parse(String((call[1] as RequestInit).body))).toEqual({ channel: 'beta' });
	});
});
