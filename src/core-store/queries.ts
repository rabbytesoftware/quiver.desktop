import { useQuery } from '@tanstack/react-query';

import type { ArrowDetailDTO } from './dtos/v0/arrow';
import type { CollectionDetailDTO, CollectionListItemDTO } from './dtos/v0/collection';

import { fetchArrowDetail, fetchCollections, fetchCollectionDetail } from './http';

export const queryKeys = {
	arrowDetail: (namespace: string) => ['arrow', 'detail', namespace] as const,
	collections: () => ['collections'] as const,
	collectionDetail: (namespace: string) => ['collection', 'detail', namespace] as const,
};

export function useArrowDetail(namespace: string) {
	return useQuery<ArrowDetailDTO>({
		queryKey: queryKeys.arrowDetail(namespace),
		queryFn: () => fetchArrowDetail(namespace),
		staleTime: Infinity,
	});
}

export function useCollections() {
	return useQuery<CollectionListItemDTO[]>({
		queryKey: queryKeys.collections(),
		queryFn: () => fetchCollections(),
		staleTime: 30_000,
	});
}

export function useCollectionDetail(namespace: string) {
	return useQuery<CollectionDetailDTO>({
		queryKey: queryKeys.collectionDetail(namespace),
		queryFn: () => fetchCollectionDetail(namespace),
		staleTime: Infinity,
	});
}
