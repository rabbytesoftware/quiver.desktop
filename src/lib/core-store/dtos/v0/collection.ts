import type { CollectionArrow, CollectionDetail, CollectionListItem, CollectionMedia } from '@/domain/collection';

export interface CollectionArrowDTO {
	/** `owner/repo@version` -- core sends no separate version field for a member arrow. */
	namespace: string;
	resolved: boolean;
	name?: string;
	description?: string;
}

export interface CollectionMediaDTO {
	icon?: string;
	banner?: string;
}

export interface CollectionListItemDTO {
	namespace: string;
	name: string;
	description?: string;
	tags?: string[];
	arrow_count: number;
	followed: boolean;
}

export interface CollectionDetailDTO {
	namespace: string;
	name: string;
	description?: string;
	url?: string;
	maintainers?: string[];
	tags?: string[];
	media?: CollectionMediaDTO;
	arrows: CollectionArrowDTO[];
	followed: boolean;
}

/** Core folds the version into `namespace` as `owner/repo@version`; this is the only place that gets split back apart. */
function parseArrowRef(namespace: string): { namespace: string; version?: string } {
	const at = namespace.lastIndexOf('@');
	if (at === -1) return { namespace };
	return { namespace: namespace.slice(0, at), version: namespace.slice(at + 1) };
}

export function toCollectionArrow(dto: CollectionArrowDTO): CollectionArrow {
	const { namespace, version } = parseArrowRef(dto.namespace);
	return { namespace, version, resolved: dto.resolved, name: dto.name, description: dto.description };
}

function toCollectionMedia(dto: CollectionMediaDTO | undefined): CollectionMedia {
	return { icon: dto?.icon, banner: dto?.banner };
}

export function toCollectionListItem(dto: CollectionListItemDTO): CollectionListItem {
	return {
		namespace: dto.namespace,
		name: dto.name,
		description: dto.description ?? '',
		tags: dto.tags ?? [],
		followed: dto.followed,
		arrowCount: dto.arrow_count,
	};
}

export function toCollectionDetail(dto: CollectionDetailDTO): CollectionDetail {
	return {
		namespace: dto.namespace,
		name: dto.name,
		description: dto.description ?? '',
		tags: dto.tags ?? [],
		followed: dto.followed,
		arrowCount: dto.arrows.length,
		url: dto.url,
		maintainers: dto.maintainers ?? [],
		media: toCollectionMedia(dto.media),
		arrows: dto.arrows.map(toCollectionArrow),
	};
}
