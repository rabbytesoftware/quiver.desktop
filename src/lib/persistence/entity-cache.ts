import { getDB } from './idb';
import type { ArrowCatalogRecord } from './schemas';

export async function upsertArrow(rec: ArrowCatalogRecord): Promise<void> {
	try {
		const db = await getDB();
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
