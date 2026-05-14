import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useArrowDetail, useCollections, useCollectionDetail, queryKeys } from './queries';
import * as http from './http';

vi.mock('./http');

function wrapper(queryClient: QueryClient) {
    return ({ children }: { children: React.ReactNode }) =>
        createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('queryKeys', () => {
    it('generates correct arrow detail key', () => {
        expect(queryKeys.arrowDetail('ns@v1')).toEqual(['arrow', 'detail', 'ns@v1']);
    });

    it('generates correct collections key', () => {
        expect(queryKeys.collections()).toEqual(['collections']);
    });

    it('generates correct collection detail key', () => {
        expect(queryKeys.collectionDetail('col/ns')).toEqual(['collection', 'detail', 'col/ns']);
    });
});

describe('useArrowDetail', () => {
    let queryClient: QueryClient;

    beforeEach(() => {
        queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
        vi.mocked(http.fetchArrowDetail).mockResolvedValue({
            namespace: 'ns@v1',
            name: 'Arrow',
            version: '1.0.0',
            description: '',
            license: '',
            state: 'ready',
            tags: [],
            installed_ref: 'v1.0.0',
            installed_at: '',
            installed_constraint: '',
            user_installed: true,
            active_run: null,
            last_return: null,
        });
    });

    it('fetches arrow detail by namespace', async () => {
        const { result } = renderHook(
            () => useArrowDetail('ns@v1'),
            { wrapper: wrapper(queryClient) }
        );
        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(result.current.data?.namespace).toBe('ns@v1');
        expect(http.fetchArrowDetail).toHaveBeenCalledWith('ns@v1');
    });
});

describe('useCollections', () => {
    let queryClient: QueryClient;

    beforeEach(() => {
        queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
        vi.mocked(http.fetchCollections).mockResolvedValue([
            { namespace: 'col/ns', name: 'Col', description: '', tags: [], arrow_count: 1, followed: false },
        ]);
    });

    it('fetches collections list', async () => {
        const { result } = renderHook(
            () => useCollections(),
            { wrapper: wrapper(queryClient) }
        );
        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(result.current.data).toHaveLength(1);
    });
});

describe('useCollectionDetail', () => {
    let queryClient: QueryClient;

    beforeEach(() => {
        queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
        vi.mocked(http.fetchCollectionDetail).mockResolvedValue({
            namespace: 'col/ns',
            name: 'Col',
            version: '1.0.0',
            description: '',
            url: '',
            maintainers: [],
            tags: [],
            media: {},
            arrows: [],
            followed: false,
        });
    });

    it('fetches collection detail by namespace', async () => {
        const { result } = renderHook(
            () => useCollectionDetail('col/ns'),
            { wrapper: wrapper(queryClient) }
        );
        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(result.current.data?.namespace).toBe('col/ns');
    });
});
