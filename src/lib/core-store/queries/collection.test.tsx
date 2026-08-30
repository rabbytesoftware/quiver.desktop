import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { collectionQueryKey, useCollectionDetail } from './collection';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('useCollectionDetail', () => {
	beforeEach(() => vi.mocked(apiFetch).mockReset());

	it('fetches by namespace and maps the detail DTO through toCollectionDetail', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			maintainers: ['rabbyte'],
			arrows: [{ namespace: 'github.com/rabbyte/minecraft@v1.21.4', resolved: true, name: 'Minecraft Server' }],
		});

		const { result } = renderHook(() => useCollectionDetail('github.com/rabbyte/game-servers'), { wrapper });

		await waitFor(() => expect(result.current.isSuccess).toBe(true));
		expect(result.current.data?.name).toBe('Game Servers');
		expect(result.current.data?.arrowCount).toBe(1);
		expect(result.current.data?.arrows[0]?.version).toBe('v1.21.4');
		expect(apiFetch).toHaveBeenCalledWith('/v0/collection/github.com%2Frabbyte%2Fgame-servers');
	});

	it('URL-encodes a namespace containing slashes as a single path segment', async () => {
		vi.mocked(apiFetch).mockResolvedValue({ namespace: 'a/b/c', name: 'C', followed: false, arrows: [] });

		renderHook(() => useCollectionDetail('a/b/c'), { wrapper });

		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith('/v0/collection/a%2Fb%2Fc'));
	});

	it('never fetches for an empty namespace', async () => {
		renderHook(() => useCollectionDetail(''), { wrapper });

		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(apiFetch).not.toHaveBeenCalled();
	});
});

describe('collectionQueryKey', () => {
	it('builds a stable key from the namespace', () => {
		expect(collectionQueryKey('github.com/rabbyte/game-servers')).toEqual([
			'collection',
			'github.com/rabbyte/game-servers',
		]);
	});
});
