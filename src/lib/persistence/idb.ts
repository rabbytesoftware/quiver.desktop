import { openDB } from 'idb';
import type { IDBPDatabase } from 'idb';

import type { QuiverDB } from './schemas';

let _db: IDBPDatabase<QuiverDB> | null = null;

export async function getDB(): Promise<IDBPDatabase<QuiverDB>> {
	if (_db) return _db;
	_db = await openDB<QuiverDB>('quiver', 1, {
		upgrade(db) {
			const arrows = db.createObjectStore('quiver_arrows', {
				keyPath: ['connectionId', 'namespace'],
			});
			arrows.createIndex('connectionId', 'connectionId');
		},
	});
	return _db;
}

export function resetDB(): void {
	_db = null;
}

export const QUIVER_CACHE_VERSION = '1';

const CACHE_VERSION_KEY = 'quiver:cache-version';

export async function wipeEntityCache(): Promise<void> {
	try {
		const db = await getDB();
		await db.clear('quiver_arrows');
	} catch {
		/* best-effort wipe */
	}
}

export async function maybeWipeOnVersionChange(): Promise<void> {
	let stored: string | null;
	try {
		stored = localStorage.getItem(CACHE_VERSION_KEY);
	} catch {
		stored = null;
	}
	if (stored === QUIVER_CACHE_VERSION) return;
	await wipeEntityCache();
	try {
		localStorage.setItem(CACHE_VERSION_KEY, QUIVER_CACHE_VERSION);
	} catch {
		/* private mode — the wipe still ran */
	}
}
