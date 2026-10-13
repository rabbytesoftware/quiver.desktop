import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { followedCollectionsQueryKey, useFollowedCollections } from './collections';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useFollowedCollections', () => {
	beforeEach(() => {
		vi.mocked(apiFetch).mockReset();
	});

	it('fetches followed collections and maps each item through toCollectionListItem', async () => {
		vi.mocked(apiFetch).mockResolvedValue([
			{
				namespace: 'guild/frosthold-pack',
				name: 'Frosthold Pack',
				description: 'Survival essentials.',
				tags: [],
				arrow_count: 14,
				followed: true,
			},
			{ namespace: 'guild/questline-plus', name: 'Questline Plus', arrow_count: 9, followed: true },
		]);

		const { result } = renderHook(() => useFollowedCollections(), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toHaveLength(2);
		expect(result.current.data?.[0]).toEqual({
			namespace: 'guild/frosthold-pack',
			name: 'Frosthold Pack',
			description: 'Survival essentials.',
			tags: [],
			followed: true,
			arrowCount: 14,
		});
		expect(result.current.data?.[1].description).toBe('');
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection?followed=true');
	});

	it('surfaces an empty list without erroring', async () => {
		vi.mocked(apiFetch).mockResolvedValue([]);

		const { result } = renderHook(() => useFollowedCollections(), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data).toEqual([]);
	});
});

describe('followedCollectionsQueryKey', () => {
	it('is a stable, namespaced key', () => {
		expect(followedCollectionsQueryKey).toEqual(['collections', 'followed']);
	});
});
