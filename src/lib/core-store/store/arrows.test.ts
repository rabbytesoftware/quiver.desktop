import { describe, it, expect, beforeEach } from 'vitest';

import { useArrowStore } from './arrows';

beforeEach(() => useArrowStore.getState().reset());

describe('setCatalog', () => {
	it('projects catalog records with neutral runtime state', () => {
		useArrowStore.getState().setCatalog([
			{
				connectionId: 'local',
				namespace: 'a@1',
				name: 'a',
				description: '',
				tags: [],
				icon: null,
				banner: null,
				version: '1',
			},
		]);
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.state).toBe('absent');
		expect(entry?.active_run).toBeNull();
	});

	// A re-seed after a reconnect replaces the set wholesale; a runtime overlay
	// already applied must survive it, or every reconnect would blank the UI.
	it('preserves runtime overlay for arrows still present', () => {
		const rec = {
			connectionId: 'local',
			namespace: 'a@1',
			name: 'a',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: '1',
		};
		useArrowStore.getState().setCatalog([rec]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		useArrowStore.getState().setCatalog([rec]);
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});
});

describe('applyRuntimeUpdate', () => {
	// A cold start paints from cache. If runtime state came from disk it would
	// claim "running" about a process the daemon may have killed hours ago.
	it('overlays runtime state without touching catalog fields', () => {
		useArrowStore.getState().setCatalog([
			{
				connectionId: 'local',
				namespace: 'a@1',
				name: 'nice name',
				description: 'd',
				tags: ['t'],
				icon: 'i',
				banner: null,
				version: '1',
			},
		]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		const entry = useArrowStore.getState().arrows.get('a@1');
		expect(entry?.state).toBe('running');
		expect(entry?.name).toBe('nice name');
		expect(entry?.icon).toBe('i');
	});

	it('ignores a runtime update for an unknown arrow', () => {
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('reset', () => {
	// Switching connections must not leave the previous backend's arrows on
	// screen while the new one seeds.
	it('clears the projection', () => {
		useArrowStore.getState().setCatalog([
			{
				connectionId: 'local',
				namespace: 'a@1',
				name: 'a',
				description: '',
				tags: [],
				icon: null,
				banner: null,
				version: '1',
			},
		]);
		useArrowStore.getState().reset();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});
