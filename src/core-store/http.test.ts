import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchArrowDetail, fetchCollections, fetchCollectionDetail } from './http';

const mockFetch = (data: unknown, ok = true) =>
    vi.fn().mockResolvedValue({
        ok,
        json: () => Promise.resolve({ success: ok, data: ok ? data : undefined, error: ok ? null : 'api error' }),
    });

afterEach(() => {
    vi.unstubAllGlobals();
});

describe('fetchArrowDetail', () => {
    it('returns parsed ArrowDetailDTO on success', async () => {
        const detail = { namespace: 'ns@v1', name: 'X', state: 'ready' };
        vi.stubGlobal('fetch', mockFetch(detail));
        const result = await fetchArrowDetail('ns@v1', 'http://localhost:6982');
        expect(result.namespace).toBe('ns@v1');
    });

    it('encodes slashes in namespace', async () => {
        vi.stubGlobal('fetch', mockFetch({ namespace: 'github.com/user/repo@v1', name: 'X', state: 'ready' }));
        await fetchArrowDetail('github.com/user/repo@v1', 'http://localhost:6982');
        const url = vi.mocked(fetch).mock.calls[0][0] as string;
        expect(url).toContain('%2F');
    });

    it('throws on api error', async () => {
        vi.stubGlobal('fetch', mockFetch(null, false));
        await expect(fetchArrowDetail('ns@v1', 'http://localhost:6982')).rejects.toThrow('api error');
    });
});

describe('fetchCollections', () => {
    it('returns list on success', async () => {
        vi.stubGlobal('fetch', mockFetch([{ namespace: 'col/ns', name: 'Col' }]));
        const result = await fetchCollections('http://localhost:6982');
        expect(result).toHaveLength(1);
    });
});

describe('fetchCollectionDetail', () => {
    it('returns detail on success', async () => {
        const detail = { namespace: 'col/ns', name: 'Col', arrows: [] };
        vi.stubGlobal('fetch', mockFetch(detail));
        const result = await fetchCollectionDetail('col/ns', 'http://localhost:6982');
        expect(result.namespace).toBe('col/ns');
    });

    it('encodes slashes in namespace', async () => {
        vi.stubGlobal('fetch', mockFetch({ namespace: 'col/ns', name: 'Col', arrows: [] }));
        await fetchCollectionDetail('col/ns', 'http://localhost:6982');
        const url = vi.mocked(fetch).mock.calls[0][0] as string;
        expect(url).toContain('%2F');
    });
});
