import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getArrowsFor, removeArrow, upsertArrow } from './entity-cache';
import { resetDB } from './idb';

const rec = (connectionId: string, namespace: string) => ({
	connectionId,
	namespace,
	name: namespace,
	description: '',
	tags: [],
	icon: null,
	banner: null,
	version: '1',
});

describe('entity-cache', () => {
	beforeEach(() => {
		// fake-indexeddb persists its backing store across `it()`s within the
		// same file; resetDB() alone only forgets idb.ts's cached connection
		// handle, not the underlying data. A fresh in-memory factory per test
		// gives real isolation (the standard fake-indexeddb reset idiom).
		globalThis.indexedDB = new IDBFactory();
		resetDB();
	});

	it('round-trips a record', async () => {
		await upsertArrow(rec('local', 'a@1'));
		expect(await getArrowsFor('local')).toHaveLength(1);
	});

	// The whole reason for the compound key: two backends must never show each
	// other's arrows. Same namespace on both sides deliberately: under a
	// `namespace`-only key the second upsert would overwrite the first (they'd
	// collide on one primary key), so this only stays green under the real
	// compound key ['connectionId', 'namespace'].
	it('keeps connections isolated', async () => {
		await upsertArrow(rec('local', 'a@1'));
		await upsertArrow(rec('remote-1', 'a@1'));
		expect((await getArrowsFor('local')).map((r) => r.namespace)).toEqual(['a@1']);
		expect((await getArrowsFor('remote-1')).map((r) => r.namespace)).toEqual(['a@1']);
	});

	it('removes only the addressed row', async () => {
		await upsertArrow(rec('local', 'a@1'));
		await upsertArrow(rec('remote-1', 'a@1'));
		// Confirms both rows actually landed independently (same namespace,
		// different connections) before the removal itself is exercised.
		expect(await getArrowsFor('local')).toHaveLength(1);
		expect(await getArrowsFor('remote-1')).toHaveLength(1);
		await removeArrow('local', 'a@1');
		expect(await getArrowsFor('local')).toEqual([]);
		expect(await getArrowsFor('remote-1')).toHaveLength(1);
	});

	// A cache is not a store. A quota error, private mode, or a stale schema
	// must degrade to a miss so the caller still runs off the live path.
	it('degrades to a no-op when IDB writes throw', async () => {
		const mod = await import('./idb');
		vi.spyOn(mod, 'getDB').mockRejectedValue(new Error('QuotaExceeded'));
		await expect(upsertArrow(rec('local', 'a@1'))).resolves.toBeUndefined();
	});

	it('degrades to an empty read when IDB reads throw', async () => {
		const mod = await import('./idb');
		vi.spyOn(mod, 'getDB').mockRejectedValue(new Error('blocked'));
		await expect(getArrowsFor('local')).resolves.toEqual([]);
	});

	it('degrades to a no-op when IDB deletes throw', async () => {
		const mod = await import('./idb');
		vi.spyOn(mod, 'getDB').mockRejectedValue(new Error('blocked'));
		await expect(removeArrow('local', 'a@1')).resolves.toBeUndefined();
	});
});
