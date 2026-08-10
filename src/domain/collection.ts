interface CollectionArrow {
	namespace: string;
	name: string;
	version?: string;
}

export interface CollectionListItem {
	namespace: string;
	name: string;
	description: string;
	arrows: CollectionArrow[];
}

export interface CollectionDetail extends CollectionListItem {
	readme: string;
}
