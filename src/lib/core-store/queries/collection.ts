import { useQuery } from '@tanstack/react-query';

import type { CollectionDetail } from '@/domain/collection';
import { apiFetch } from '@/lib/transport/api';

import type { CollectionDetailDTO } from '../dtos/v0/collection';
import { toCollectionDetail } from '../dtos/v0/collection';

export function collectionQueryKey(namespace: string) {
	return ['collection', namespace] as const;
}

export function useCollectionDetail(namespace: string) {
	return useQuery<CollectionDetail>({
		queryKey: collectionQueryKey(namespace),
		queryFn: () =>
			apiFetch<CollectionDetailDTO>(`/v0/collection/${encodeURIComponent(namespace)}`).then(toCollectionDetail),
		// A bare `/collection/` route (an empty splat) has nothing to fetch --
		// don't fire `/v0/collection/` at the backend for it.
		enabled: namespace.length > 0,
	});
}
