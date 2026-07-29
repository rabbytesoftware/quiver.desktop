import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { useInstall, useUninstall, useStop, useExecute } from './runtime';

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

describe('useInstall', () => {
	it('POSTs to /v0/runtime/:ns/install with a JSON variables body', async () => {
		const { result } = renderHook(() => useInstall(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'ns@v1', variables: { KEY: 'val' } }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/ns%40v1/install', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ variables: { KEY: 'val' } }),
		});
	});

	it('defaults variables to an empty object', async () => {
		const { result } = renderHook(() => useInstall(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
		expect(apiFetch).toHaveBeenCalledWith(
			'/v0/runtime/ns%40v1/install',
			expect.objectContaining({ body: JSON.stringify({ variables: {} }) })
		);
	});
});

describe('useUninstall', () => {
	it('POSTs to /v0/runtime/:ns/uninstall', async () => {
		const { result } = renderHook(() => useUninstall(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/ns%40v1/uninstall', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ variables: {} }),
		});
	});
});

describe('useStop', () => {
	it('POSTs to /v0/runtime/:ns/stop', async () => {
		const { result } = renderHook(() => useStop(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/ns%40v1/stop', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ variables: {} }),
		});
	});
});

describe('useExecute', () => {
	it('POSTs to /v0/runtime/:ns/:method using the caller-supplied method', async () => {
		const { result } = renderHook(() => useExecute(), { wrapper: wrapper() });
		await act(() => result.current.mutateAsync({ namespace: 'ns@v1', method: '_execute', variables: { a: 'b' } }));
		expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/ns%40v1/_execute', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ variables: { a: 'b' } }),
		});
	});
});
