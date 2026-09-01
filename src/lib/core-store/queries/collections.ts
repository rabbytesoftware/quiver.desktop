import { useQuery } from '@tanstack/react-query';

import type { CollectionListItem } from '@/domain/collection';
import { apiFetch } from '@/lib/transport/api';

import type { CollectionListItemDTO } from '../dtos/v0/collection';
import { toCollectionListItem } from '../dtos/v0/collection';

export const followedCollectionsQueryKey = ['collections', 'followed'] as const;

/** `GET /v0/collection?followed=true` -- the collections the user has already opted into, for Home and the full Collections page. */
export function useFollowedCollections() {
	return useQuery<CollectionListItem[]>({
		queryKey: followedCollectionsQueryKey,
		queryFn: () =>
			apiFetch<CollectionListItemDTO[]>('/v0/collection?followed=true').then((items) =>
				items.map(toCollectionListItem)
			),
	});
}
