import 'fake-indexeddb/auto';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDB, maybeWipeOnVersionChange, QUIVER_CACHE_VERSION, resetDB, wipeEntityCache } from './idb';

// The `localStorage` environment fix this file used to carry inline now lives
// suite-wide at `src/__mocks__/setup-local-storage.ts`, wired in via
// `vitest.config.ts`'s `setupFiles` — see that file for why it's needed.

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

	// Coverage-driven / review-driven: a prior mutation pass only exercised
	// `wipeEntityCache` by deleting the `db.clear()` call outright, which also
	// changes the happy-path result and so isn't a test of the `catch` at all.
	// This makes the store itself throw so the *only* thing under test is
	// whether the surrounding try/catch swallows it.
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
			// Best-effort means "leave it alone on failure", not "clear anyway".
			expect(await db.getAll('quiver_arrows')).toHaveLength(1);
		} finally {
			clearSpy.mockRestore();
		}
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

	// Coverage/review-driven: a prior mutation pass only exercised the
	// version-compare `===`→`!==` flip, which tests the branch logic, not the
	// `localStorage.setItem` catch. This forces a genuine mismatch (so the wipe
	// must run) while `setItem` itself throws, isolating that the surrounding
	// try/catch is what keeps the function resolving — "private mode — the
	// wipe still ran", per the source comment.
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
