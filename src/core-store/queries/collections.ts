import { useQuery } from '@tanstack/react-query';

import type { CollectionDetailDTO, CollectionListItemDTO } from '../dtos/v0/collection';

import { fetchCollections, fetchCollectionDetail } from '../http';
import { queryKeys } from './index';

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
