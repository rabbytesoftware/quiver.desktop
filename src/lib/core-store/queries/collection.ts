import { useQuery } from '@tanstack/react-query';

import type { CollectionDetail } from '@/domain/collection';
import { apiFetch } from '@/lib/transport/api';

import type { CollectionDetailDTO } from '../dtos/v0/collection';
import { toCollectionDetail } from '../dtos/v0/collection';

export function useCollectionDetail(namespace: string) {
	return useQuery<CollectionDetail>({
		queryKey: ['collection', namespace],
		queryFn: () =>
			apiFetch<CollectionDetailDTO>(`/v0/collection/${encodeURIComponent(namespace)}`).then(toCollectionDetail),
	});
}
