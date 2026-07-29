import { getDB } from './idb';
import type { ArrowCatalogRecord } from './schemas';

// Strictly best-effort. A missing object store (stale schema), a quota error,
// or a private-mode restriction degrades to a no-op write or an empty read, so
// the caller still functions off the live GET/WS path. A cache that can break
// the data flow is worse than no cache.

export async function upsertArrow(rec: ArrowCatalogRecord): Promise<void> {
	try {
		const db = await getDB();
		// keyPath is ['connectionId','namespace'], so no explicit key argument.
		await db.put('quiver_arrows', rec);
	} catch {
		/* best-effort cache write */
	}
}

export async function getArrowsFor(connectionId: string): Promise<ArrowCatalogRecord[]> {
	try {
		const db = await getDB();
		return await db.getAllFromIndex('quiver_arrows', 'connectionId', connectionId);
	} catch {
		return [];
	}
}

export async function removeArrow(connectionId: string, namespace: string): Promise<void> {
	try {
		const db = await getDB();
		await db.delete('quiver_arrows', [connectionId, namespace]);
	} catch {
		/* best-effort cache delete */
	}
}
