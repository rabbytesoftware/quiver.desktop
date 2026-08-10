import 'fake-indexeddb/auto';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDB, maybeWipeOnVersionChange, QUIVER_CACHE_VERSION, resetDB, wipeEntityCache } from './idb';

describe('idb', () => {
	beforeEach(() => {
		resetDB();
		localStorage.clear();
	});

	it('creates the arrows store keyed by connection and namespace', async () => {
		const db = await getDB();
		expect(db.objectStoreNames.contains('quiver_arrows')).toBe(true);
	});

	it('returns the same handle on repeated calls', async () => {
		expect(await getDB()).toBe(await getDB());
	});

	it('wipes the entity cache', async () => {
		const db = await getDB();
		await db.put('quiver_arrows', {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		});
		await wipeEntityCache();
		expect(await db.getAll('quiver_arrows')).toEqual([]);
	});

	it('degrades to a no-op when clearing the store throws', async () => {
		const db = await getDB();
		await db.put('quiver_arrows', {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		});
		const clearSpy = vi.spyOn(db, 'clear').mockRejectedValue(new Error('blocked'));
		try {
			await expect(wipeEntityCache()).resolves.toBeUndefined();
			expect(await db.getAll('quiver_arrows')).toHaveLength(1);
		} finally {
			clearSpy.mockRestore();
		}
	});

	it('wipes on a version mismatch and records the new version', async () => {
		localStorage.setItem('quiver:cache-version', 'ancient');
		await maybeWipeOnVersionChange();
		expect(localStorage.getItem('quiver:cache-version')).toBe(QUIVER_CACHE_VERSION);
	});

	it('is a no-op when the version already matches', async () => {
		localStorage.setItem('quiver:cache-version', QUIVER_CACHE_VERSION);
		const db = await getDB();
		await db.put('quiver_arrows', {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		});
		await maybeWipeOnVersionChange();
		expect(await db.getAll('quiver_arrows')).toHaveLength(1);
	});

	it('treats an unreadable localStorage as a version mismatch', async () => {
		const db = await getDB();
		await db.put('quiver_arrows', {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		});
		const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')!;
		Object.defineProperty(globalThis, 'localStorage', {
			value: {
				getItem(): string | null {
					throw new Error('SecurityError');
				},
				setItem(): void {},
				removeItem(): void {},
				clear(): void {},
				key(): string | null {
					return null;
				},
				length: 0,
			} satisfies Storage,
			configurable: true,
		});
		try {
			await maybeWipeOnVersionChange();
			expect(await db.getAll('quiver_arrows')).toEqual([]);
		} finally {
			Object.defineProperty(globalThis, 'localStorage', original);
		}
	});

	it('still wipes even when localStorage.setItem throws', async () => {
		const db = await getDB();
		await db.put('quiver_arrows', {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		});
		const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')!;
		Object.defineProperty(globalThis, 'localStorage', {
			value: {
				getItem(): string | null {
					return 'ancient';
				},
				setItem(): void {
					throw new Error('QuotaExceededError');
				},
				removeItem(): void {},
				clear(): void {},
				key(): string | null {
					return null;
				},
				length: 0,
			} satisfies Storage,
			configurable: true,
		});
		try {
			await expect(maybeWipeOnVersionChange()).resolves.toBeUndefined();
			expect(await db.getAll('quiver_arrows')).toEqual([]);
		} finally {
			Object.defineProperty(globalThis, 'localStorage', original);
		}
	});
});
