import 'fake-indexeddb/auto';
import { beforeEach, describe, expect, it } from 'vitest';

import { getDB, maybeWipeOnVersionChange, QUIVER_CACHE_VERSION, resetDB, wipeEntityCache } from './idb';

// Environment shim, not part of the design under test: this Node/Vitest combo
// ships a built-in global `localStorage` (Node's Web Storage API, unflagged)
// that shadows jsdom's real implementation and is missing methods such as
// `.clear()`. Vitest's jsdom environment skips copying a jsdom global that
// already exists on `globalThis`, so without this every test below throws on
// the very first `localStorage.clear()` call. Rebind to jsdom's own Storage.
const jsdomWindow = (globalThis as unknown as { jsdom?: { window: { localStorage: Storage } } }).jsdom?.window;
if (jsdomWindow) {
	Object.defineProperty(globalThis, 'localStorage', {
		get: () => jsdomWindow.localStorage,
		configurable: true,
	});
}

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

	// First run has no recorded version, so it must wipe — otherwise a cache
	// written by an older DTO shape survives into a build that cannot read it.
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

	// Coverage-driven, beyond the brief's Step 1 file: `localStorage.getItem`
	// throwing (private-mode Safari, or storage torn down mid-navigation) must
	// be treated the same as "no recorded version" — i.e. wipe — not silently
	// skipped. Without this test the catch's `stored = null` assignment is dark.
	//
	// `vi.spyOn(localStorage, 'getItem')` does NOT work here: jsdom's Storage
	// is backed by a Proxy whose own `getItem`/`setItem` win over anything
	// vi.spyOn assigns onto the object, so the mock silently never fires and
	// the "real" getItem still answers. Confirmed by running this with only
	// the spyOn approach: it stayed green even when the assertion demanded a
	// wipe that hadn't happened — a vacuous test. Swapping the whole
	// `localStorage` global for a plain object sidesteps that Proxy entirely.
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
});
