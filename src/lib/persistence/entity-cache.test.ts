import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getArrow, getArrowsFor, removeArrow, upsertArrow } from './entity-cache';
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
		globalThis.indexedDB = new IDBFactory();
		resetDB();
	});

	it('round-trips a record', async () => {
		await upsertArrow(rec('local', 'a@1'));
		expect(await getArrowsFor('local')).toHaveLength(1);
	});

	it('keeps connections isolated', async () => {
		await upsertArrow(rec('local', 'a@1'));
		await upsertArrow(rec('remote-1', 'a@1'));
		expect((await getArrowsFor('local')).map((r) => r.namespace)).toEqual(['a@1']);
		expect((await getArrowsFor('remote-1')).map((r) => r.namespace)).toEqual(['a@1']);
	});

	it('removes only the addressed row', async () => {
		await upsertArrow(rec('local', 'a@1'));
		await upsertArrow(rec('remote-1', 'a@1'));
		expect(await getArrowsFor('local')).toHaveLength(1);
		expect(await getArrowsFor('remote-1')).toHaveLength(1);
		await removeArrow('local', 'a@1');
		expect(await getArrowsFor('local')).toEqual([]);
		expect(await getArrowsFor('remote-1')).toHaveLength(1);
	});

	it('reads one record by connection and identity', async () => {
		await upsertArrow(rec('local', 'a@stable'));
		expect((await getArrow('local', 'a@stable'))?.version).toBe('1');
		expect(await getArrow('remote-1', 'a@stable')).toBeUndefined();
	});

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

	it('degrades to no record when an IDB single read throws', async () => {
		const mod = await import('./idb');
		vi.spyOn(mod, 'getDB').mockRejectedValue(new Error('blocked'));
		await expect(getArrow('local', 'a@1')).resolves.toBeUndefined();
	});

	it('degrades to a no-op when IDB deletes throw', async () => {
		const mod = await import('./idb');
		vi.spyOn(mod, 'getDB').mockRejectedValue(new Error('blocked'));
		await expect(removeArrow('local', 'a@1')).resolves.toBeUndefined();
	});
});
