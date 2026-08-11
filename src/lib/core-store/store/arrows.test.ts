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

	it('preserves runtime overlay for arrows still present', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		useArrowStore.getState().setCatalog([rec]);
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

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

	it('does not let a namespace inherit its previous overlay after leaving and rejoining the catalog', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		useArrowStore.getState().setCatalog([catalogRecord('b@1')]);
		useArrowStore.getState().setCatalog([rec, catalogRecord('b@1')]);
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('absent');
	});
});

describe('applyRuntimeUpdate', () => {
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

	it('preserves the last known outcome when a later frame omits it, including across a subsequent setCatalog', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
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
		useArrowStore.getState().setCatalog([rec]);
		expect(useArrowStore.getState().arrows.get('a@1')?.last_return?.outcome).toBe('success');
	});

	it('starts out loading, so a cold rail never claims the library is empty', () => {
		expect(useArrowStore.getState().catalog).toBe('loading');
	});

	it('marks the catalog ready once records land, even when there are none', () => {
		useArrowStore.getState().setCatalog([]);
		expect(useArrowStore.getState().catalog).toBe('ready');
	});

	it('marks the catalog errored when the seed fails', () => {
		useArrowStore.getState().setCatalogError();
		expect(useArrowStore.getState().catalog).toBe('error');
	});

	it('returns to loading on reset, so a restarting core does not read as empty', () => {
		useArrowStore.getState().setCatalog([]);
		useArrowStore.getState().reset();
		expect(useArrowStore.getState().catalog).toBe('loading');
	});

	it('clears an earlier error once a later seed succeeds', () => {
		useArrowStore.getState().setCatalogError();
		useArrowStore.getState().setCatalog([]);
		expect(useArrowStore.getState().catalog).toBe('ready');
	});

	it('ignores a runtime update for an unknown arrow', () => {
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('seedInitialState', () => {
	it('paints the initial state from the seed for an arrow with no overlay yet', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

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

	it('allows a newer seed to overwrite an older seed-derived state', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'ready', active_run: null, last_return: null });
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	it('resolves the overlay before storing it for seedInitialState too, so a subsequent setCatalog does not wipe a preserved outcome', () => {
		const rec = catalogRecord('a@1');
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore.getState().seedInitialState({
			namespace: 'a@1',
			state: 'ready',
			active_run: null,
			last_return: { method: 'install', outcome: 'success' },
		});
		useArrowStore
			.getState()
			.seedInitialState({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.last_return?.outcome).toBe('success');
		useArrowStore.getState().setCatalog([rec]);
		expect(useArrowStore.getState().arrows.get('a@1')?.last_return?.outcome).toBe('success');
	});
});

describe('reset', () => {
	it('clears the projection', () => {
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		useArrowStore.getState().reset();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});
