// The mock driving the REAL data layer, end to end.
//
// Everything else in this suite tests the mock in isolation. This one wires it
// to `setupListeners` — the actual startup path, with its generation counter,
// its cache seed, its entity stream and its Zustand projection — because the
// bugs that matter here live in the seam between them, and neither side's own
// tests can see them: the mock's pass with no store attached, and the store's
// pass against a hand-written stub that never has to boot.

// jsdom ships no IndexedDB, and this suite goes through the real cache. Without
// it every `getArrowsFor` falls through `entity-cache`'s best-effort catch and
// answers `[]` — so the projection comes out empty for a reason that has
// nothing to do with the code under test. Imported per-file because that is
// this repo's convention; it is not in `setupFiles`.
import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockBackend, type MockRuntime } from './index';

let mock: MockRuntime;

/**
 * Boot the app on a FRESH module graph, with the mock installed into that same
 * graph.
 *
 * `setupListeners` keeps its generation counter, its in-flight count and its
 * stream disposers in closure state created once per call, and it is written to
 * run exactly once per app lifetime. Sharing one graph across cases would let
 * the first test's streams stay live against a disposed world and answer for
 * the second's.
 *
 * Everything must come from the same fresh graph — `installBackend` included.
 * Installing into the outer module instance would leave the freshly-imported
 * listeners talking to `realBackend`, which reaches for Tauri IPC that does not
 * exist here and fails several layers from the cause.
 */
async function boot() {
	vi.resetModules();

	const { installBackend } = await import('@/lib/transport/backend');
	installBackend(mock.backend);

	const { resetDB } = await import('@/lib/persistence/idb');
	resetDB();

	const { setupListeners } = await import('@/lib/core-store/listeners');
	const { useArrowStore } = await import('@/lib/core-store/store/arrows');
	const { useStatusStore } = await import('@/lib/core-store/store/status');
	const { apiFetch } = await import('@/lib/transport/api');

	await setupListeners();
	// REAL timers here, deliberately. The seed round-trips through IndexedDB,
	// and `fake-indexeddb` drives its transactions on the macrotask queue —
	// under fake timers those never run, `getArrowsFor` answers [] from its own
	// best-effort catch, and the projection ends up empty for a reason that has
	// nothing to do with the code under test. Costs the mock's 400ms boot.
	await realDelay(700);

	return { useArrowStore, useStatusStore, apiFetch };
}

/** A wait the fake-timer switch below cannot swallow. */
function realDelay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

beforeEach(() => {
	// fake-indexeddb keeps its backing store across `it()`s in a file; a fresh
	// factory is the standard idiom for real isolation between them.
	globalThis.indexedDB = new IDBFactory();
	mock = createMockBackend('normal');
});

afterEach(() => {
	mock.dispose();
	vi.useRealTimers();
});

describe('booting the app against the mock', () => {
	it('reaches ready', async () => {
		const { useStatusStore } = await boot();
		expect(useStatusStore.getState().status).toBe('ready');
	});

	// The one that matters: rows reaching IndexedDB is NOT the same as rows
	// reaching the projection, and the app renders the projection.
	it('fills the arrow projection, not just the cache', async () => {
		const { useArrowStore } = await boot();
		expect(useArrowStore.getState().arrows.size).toBeGreaterThan(0);
	});

	it('paints the seeded state rather than defaulting everything to absent', async () => {
		const { useArrowStore } = await boot();
		const arrows = useArrowStore.getState().arrows;
		// If `toInitialRuntimeUpdates` were not applied, every entry would sit on
		// the store's neutral default and this would be the single value {absent}.
		expect(new Set([...arrows.values()].map((a) => a.state)).size).toBeGreaterThan(1);
		expect(arrows.get('github.com/rabbyte/minecraft@v1.21.4')?.state).toBe('running');
	});

	it('carries media through the nested DTO shape', async () => {
		const { useArrowStore } = await boot();
		const minecraft = useArrowStore.getState().arrows.get('github.com/rabbyte/minecraft@v1.21.4');
		expect(minecraft?.icon).toMatch(/^data:image\/svg\+xml,/);
	});

	it('shows the library, not the whole searchable universe', async () => {
		const { useArrowStore } = await boot();
		const arrows = useArrowStore.getState().arrows;
		expect(arrows.get('github.com/rabbyte/mariadb@v11.6.2')).toBeUndefined();
	});
});

describe('a live install, through the whole stack', () => {
	it('moves the projected state from absent to ready as the timeline runs', async () => {
		const { useArrowStore, apiFetch } = await boot();
		const ns = 'github.com/rabbyte/postgres@v17.2';
		mock.world.arrows.get(ns)!.state = 'absent';

		// Fake timers only from HERE. The install's step timeline is pure
		// in-memory state plus `/v0/runtime` frames — it never touches the cache,
		// so freezing the clock now cannot starve IndexedDB the way it would
		// during the seed above. Four and a bit seconds of wall clock become
		// instant.
		vi.useFakeTimers();

		await apiFetch(`/v0/runtime/${encodeURIComponent(ns)}/install`, { method: 'POST' });
		await vi.advanceTimersByTimeAsync(50);
		expect(useArrowStore.getState().arrows.get(ns)!.state).toBe('installing');
		expect(useArrowStore.getState().arrows.get(ns)!.active_run?.steps).toHaveLength(5);

		await vi.advanceTimersByTimeAsync(700 * 7);
		expect(useArrowStore.getState().arrows.get(ns)!.state).toBe('ready');
		expect(useArrowStore.getState().arrows.get(ns)!.last_return?.outcome).toBe('success');
	});
});
