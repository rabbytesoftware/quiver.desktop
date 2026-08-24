import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockBackend, type MockRuntime } from './index';

let mock: MockRuntime;

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
	await booted(useStatusStore, useArrowStore);

	return { useArrowStore, useStatusStore, apiFetch };
}

function realDelay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Waits for the boot to land rather than guessing how long it takes. This
 * replaced a flat `realDelay(700)`, which was enough on an idle machine and not
 * under coverage instrumentation: `carries media through the nested DTO shape`
 * failed about one run in eight, reading an arrow the projection had not written
 * yet, while `npm test` stayed green because it runs faster than `test:coverage`.
 *
 * Settling is two conditions, not one. `status: ready` says the daemon answered;
 * a catalog size that stops moving says the projection has finished draining the
 * seed behind it. Waiting on `ready` alone reintroduces the same race.
 */
async function booted(
	useStatusStore: { getState: () => { status: string } },
	useArrowStore: { getState: () => { arrows: Map<string, unknown> } }
): Promise<void> {
	const deadline = Date.now() + 10_000;
	let previous = -1;
	let stable = 0;

	while (Date.now() < deadline) {
		const size = useArrowStore.getState().arrows.size;
		const ready = useStatusStore.getState().status === 'ready';

		stable = ready && size > 0 && size === previous ? stable + 1 : 0;
		if (stable >= 3) return;

		previous = size;
		await realDelay(20);
	}

	throw new Error('the mock never booted: status or catalog never settled');
}

beforeEach(() => {
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

	it('fills the arrow projection, not just the cache', async () => {
		const { useArrowStore } = await boot();
		expect(useArrowStore.getState().arrows.size).toBeGreaterThan(0);
	});

	it('paints the seeded state rather than defaulting everything to absent', async () => {
		const { useArrowStore } = await boot();
		const arrows = useArrowStore.getState().arrows;
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
