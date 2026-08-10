import type { CollectionListItem, CollectionDetail } from '@/domain/collection';

interface CollectionArrowDTO {
	namespace: string;
	name: string;
	version?: string;
}

export interface CollectionListItemDTO {
	namespace: string;
	name: string;
	description?: string;
	arrows: CollectionArrowDTO[];
}

export interface CollectionDetailDTO extends CollectionListItemDTO {
	readme?: string;
}

export function toCollectionListItem(dto: CollectionListItemDTO): CollectionListItem {
	return {
		namespace: dto.namespace,
		name: dto.name,
		description: dto.description ?? '',
		arrows: dto.arrows,
	};
}

export function toCollectionDetail(dto: CollectionDetailDTO): CollectionDetail {
	return {
		...toCollectionListItem(dto),
		readme: dto.readme ?? '',
	};
}
