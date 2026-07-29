import { describe, it, expect, beforeEach } from 'vitest';

import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

import { useArrowStore } from './arrows';

beforeEach(() => useArrowStore.getState().reset());

const catalogRecord = (namespace: string): ArrowCatalogRecord => ({
	connectionId: 'local',
	namespace,
	name: namespace,
	description: '',
	tags: [],
	icon: null,
	banner: null,
	version: '1',
});

describe('setCatalog', () => {
	it('projects catalog records with neutral runtime state', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.state).toBe('absent');
		expect(entry?.active_run).toBeNull();
	});

	// A re-seed after a reconnect replaces the set wholesale; a runtime overlay
	// already applied must survive it, or every reconnect would blank the UI.
	it('preserves runtime overlay for arrows still present', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		useArrowStore.getState().setCatalog([rec]);
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	// Same claim as above, but for the two fields a prior mutation round found
	// completely unpinned: both `active_run` and `last_return` mutated to a
	// hardcoded `null` in toEntry's overlay branch left the whole suite green.
	it('preserves a non-null active_run and last_return across a reseed', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'a@1',
			state: 'running',
			active_run: { method: 'execute', variables: {}, steps: [] },
			last_return: { method: 'install', outcome: 'success' },
		});
		useArrowStore.getState().setCatalog([rec]);
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.active_run?.method).toBe('execute');
		expect(entry?.last_return?.outcome).toBe('success');
	});

	// Minor fix: an uninstalled-then-reinstalled namespace must not inherit its
	// previous life's overlay — otherwise `runtime` also grows unboundedly
	// across every reconnect for every namespace that ever left the catalog.
	it('does not let a namespace inherit its previous overlay after leaving and rejoining the catalog', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		// 'a@1' leaves the catalog (uninstalled) ...
		useArrowStore.getState().setCatalog([catalogRecord('b@1')]);
		// ... then comes back (reinstalled) — a fresh row, not the old 'running' one.
		useArrowStore.getState().setCatalog([rec, catalogRecord('b@1')]);
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('absent');
	});
});

describe('applyRuntimeUpdate', () => {
	// A cold start paints from cache. If runtime state came from disk it would
	// claim "running" about a process the daemon may have killed hours ago.
	it('overlays runtime state without touching catalog fields', () => {
		useArrowStore
			.getState()
			.setCatalog([{ ...catalogRecord('a@1'), name: 'nice name', description: 'd', tags: ['t'], icon: 'i' }]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.state).toBe('running');
		expect(entry?.name).toBe('nice name');
		expect(entry?.icon).toBe('i');
	});

	// The overlay's whole reason to exist is patching active_run/last_return —
	// a mutation blanking both to null on every update left 126/126 green,
	// because every existing test only ever passed null for them.
	it('overlays a non-null active_run and a non-null last_return', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'a@1',
			state: 'running',
			active_run: { method: 'execute', variables: { KEY: 'val' }, steps: [] },
			last_return: { method: 'install', outcome: 'success' },
		});
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.active_run).toEqual({ method: 'execute', variables: { KEY: 'val' }, steps: [] });
		expect(entry?.last_return).toEqual({ method: 'install', outcome: 'success' });
	});

	// A state-only transition frame (e.g. 'ready' -> 'running') carries no
	// last_return of its own — it must not wipe the last known outcome from
	// the UI. Deleted along with this behaviour during the original rewrite;
	// restored per review finding 3 (fix round 1).
	it('preserves the last known outcome when a later frame omits it', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'a@1',
			state: 'ready',
			active_run: null,
			last_return: { method: 'install', outcome: 'success' },
		});
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.last_return?.outcome).toBe('success');
	});

	it('ignores a runtime update for an unknown arrow', () => {
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('seedInitialState', () => {
	// Neither /v0/arrow nor /v0/runtime push anything on connect — this is the
	// only source for a correct initial paint (review finding 5, fix round 1).
	it('paints the initial state from the seed for an arrow with no overlay yet', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	// A reconnect's fresh GET can lag behind an in-flight live transition —
	// seeding must never downgrade an overlay a `/v0/runtime` frame already
	// established, or a reseed could stomp genuinely-fresher live data.
	it('does not overwrite an overlay a live runtime frame already established', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'ready', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	it('ignores a seed for an unknown arrow', () => {
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('reset', () => {
	// Switching connections must not leave the previous backend's arrows on
	// screen while the new one seeds.
	it('clears the projection', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore.getState().reset();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});
