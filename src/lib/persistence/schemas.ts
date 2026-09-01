import type { DBSchema } from 'idb';

export interface ArrowCatalogRecord {
	connectionId: string;
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	version: string;
	last_used_at?: string | null;
}

export interface QuiverDB extends DBSchema {
	quiver_arrows: {
		key: [string, string];
		value: ArrowCatalogRecord;
		indexes: { connectionId: string };
	};
}
