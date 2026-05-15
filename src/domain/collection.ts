export interface CollectionMedia {
	icon?: string;
	banner?: string;
}

export interface CollectionArrow {
	namespace: string;
	resolved: boolean;
	name?: string;
	version?: string;
	description?: string;
}

export interface CollectionListItemDTO {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	arrow_count: number;
	followed: boolean;
}

export interface CollectionDetailDTO {
	namespace: string;
	name: string;
	version: string;
	description: string;
	url: string;
	maintainers: string[];
	tags: string[];
	media: CollectionMedia;
	arrows: CollectionArrow[];
	followed: boolean;
}
