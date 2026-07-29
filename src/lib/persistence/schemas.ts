import type { DBSchema } from 'idb';

/**
 * One arrow as persisted. CATALOG FIELDS ONLY — deliberately.
 *
 * `state`, `active_run` and `last_return` are facts about right now. Writing
 * them to disk means a cold start paints a lie about a process the daemon may
 * have killed hours ago. They live in memory only; the store composes them onto
 * this record. See design doc §5.4.
 */
export interface ArrowCatalogRecord {
	connectionId: string;
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	version: string;
}

export interface QuiverDB extends DBSchema {
	quiver_arrows: {
		// Compound key: quiver has many backends, so a cache row is only
		// meaningful alongside the connection it came from. Crowbar never
		// needed this, having exactly one daemon.
		key: [string, string];
		value: ArrowCatalogRecord;
		indexes: { connectionId: string };
	};
}
