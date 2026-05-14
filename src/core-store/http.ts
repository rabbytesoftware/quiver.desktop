import type { ArrowDetailDTO } from '@/domain/arrow';
import type { CollectionDetailDTO, CollectionListItemDTO } from '@/domain/collection';

const DEFAULT_BASE_URL = 'http://localhost:6982';

interface ApiEnvelope<T> {
    success: boolean;
    error: string | null;
    data?: T;
}

async function apiFetch<T>(path: string, baseUrl = DEFAULT_BASE_URL): Promise<T> {
    const res = await fetch(`${baseUrl}${path}`);
    const envelope: ApiEnvelope<T> = await res.json();
    if (!envelope.success || envelope.data === undefined) {
        throw new Error(envelope.error ?? 'Unknown error');
    }
    return envelope.data;
}

export function fetchArrowDetail(namespace: string, baseUrl?: string): Promise<ArrowDetailDTO> {
    const encoded = namespace.replace(/\//g, '%2F');
    return apiFetch<ArrowDetailDTO>(`/v0/arrow/${encoded}`, baseUrl);
}

export function fetchCollections(baseUrl?: string): Promise<CollectionListItemDTO[]> {
    return apiFetch<CollectionListItemDTO[]>('/v0/collection', baseUrl);
}

export function fetchCollectionDetail(namespace: string, baseUrl?: string): Promise<CollectionDetailDTO> {
    const encoded = namespace.replace(/\//g, '%2F');
    return apiFetch<CollectionDetailDTO>(`/v0/collection/${encoded}`, baseUrl);
}
