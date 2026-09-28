import type { DBSchema } from 'idb';

import type { ArrowOriginFields } from '@/domain/arrow';

/** `origin`/`confidence` are absent on a row cached before quiver.core reported them, which reads as declared. */
export interface ArrowCatalogRecord extends ArrowOriginFields {
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
