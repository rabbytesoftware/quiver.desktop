import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { ApiError, apiFetch } from '@/lib/transport/api';

import { useHome } from './home';

vi.mock('@/lib/transport/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('@/lib/transport/api')>()),
	apiFetch: vi.fn(),
}));

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

const shelf = { id: 'popular', title: 'Popular', refreshed_at: null, arrows: [] };

describe('useHome', () => {
	beforeEach(() => {
		vi.mocked(apiFetch).mockReset();
	});

	it('maps the snapshot', async () => {
		vi.mocked(apiFetch).mockResolvedValue({ shelves: [shelf], refreshing: false });
		const { result } = renderHook(() => useHome(), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.shelves[0].title).toBe('Popular');
		expect(apiFetch).toHaveBeenCalledWith('/v0/home');
	});

	it('reads a 404 as no shelves', async () => {
		vi.mocked(apiFetch).mockRejectedValue(new ApiError('not found', 404));
		const { result } = renderHook(() => useHome(), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual({ shelves: [], refreshing: false });
	});

	it('surfaces any other failure', async () => {
		vi.mocked(apiFetch).mockRejectedValue(new ApiError('boom', 500));
		const { result } = renderHook(() => useHome(), { wrapper });

		await waitFor(() => expect(result.current.isError).toBe(true));
	});

	it('polls every 3s while refreshing', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		try {
			vi.mocked(apiFetch).mockResolvedValue({ shelves: [], refreshing: true });
			const { result } = renderHook(() => useHome(), { wrapper });
			await waitFor(() => expect(result.current.isSuccess).toBe(true));

			await vi.advanceTimersByTimeAsync(3100);
			expect(vi.mocked(apiFetch).mock.calls.length).toBeGreaterThanOrEqual(2);
		} finally {
			vi.useRealTimers();
		}
	});

	it('does not poll once idle', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		try {
			vi.mocked(apiFetch).mockResolvedValue({ shelves: [], refreshing: false });
			const { result } = renderHook(() => useHome(), { wrapper });
			await waitFor(() => expect(result.current.isSuccess).toBe(true));

			await vi.advanceTimersByTimeAsync(10_000);
			expect(apiFetch).toHaveBeenCalledTimes(1);
		} finally {
			vi.useRealTimers();
		}
	});
});
