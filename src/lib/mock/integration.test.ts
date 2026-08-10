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
	await realDelay(700);

	return { useArrowStore, useStatusStore, apiFetch };
}

function realDelay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
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
