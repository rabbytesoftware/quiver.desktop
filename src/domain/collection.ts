export interface CollectionArrow {
	namespace: string;
	version?: string;
	resolved: boolean;
	name?: string;
	description?: string;
}

export interface CollectionListItem {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	followed: boolean;
	arrowCount: number;
}

export interface CollectionMedia {
	icon?: string;
	banner?: string;
}

export interface CollectionDetail extends CollectionListItem {
	url?: string;
	maintainers: string[];
	media: CollectionMedia;
	arrows: CollectionArrow[];
}
