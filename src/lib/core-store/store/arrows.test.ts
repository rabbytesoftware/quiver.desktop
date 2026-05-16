import { describe, it, expect, beforeEach } from 'vitest';
import { useArrowStore } from './arrows';
import type { ArrowEntry } from '@/domain/arrow';

const entry = (): ArrowEntry => ({
	namespace:   'github.com/foo/bar@v1',
	name:        'bar',
	description: 'A bar arrow',
	tags:        ['cli'],
	icon:        null,
	banner:      null,
	version:     '1.0.0',
	state:       'ready',
	active_run:  null,
	last_return: null,
});

beforeEach(() => useArrowStore.getState().resetArrows());

describe('upsertArrow', () => {
	it('adds a new entry', () => {
		useArrowStore.getState().upsertArrow(entry());
		expect(useArrowStore.getState().arrows.size).toBe(1);
	});

	it('overwrites an existing entry', () => {
		useArrowStore.getState().upsertArrow(entry());
		useArrowStore.getState().upsertArrow({ ...entry(), state: 'running' });
		expect(useArrowStore.getState().arrows.get('github.com/foo/bar@v1')?.state).toBe('running');
	});
});

describe('removeArrow', () => {
	it('deletes by namespace', () => {
		useArrowStore.getState().upsertArrow(entry());
		useArrowStore.getState().removeArrow('github.com/foo/bar@v1');
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('applyRuntimeUpdate', () => {
	it('patches state and runtime fields', () => {
		useArrowStore.getState().upsertArrow(entry());
		useArrowStore.getState().applyRuntimeUpdate({
			namespace:   'github.com/foo/bar@v1',
			state:       'running',
			active_run:  { method: 'execute', variables: {}, steps: [] },
			last_return: null,
		});
		const e = useArrowStore.getState().arrows.get('github.com/foo/bar@v1')!;
		expect(e.state).toBe('running');
		expect(e.active_run?.method).toBe('execute');
	});

	it('preserves existing last_return when update sends null', () => {
		useArrowStore.getState().upsertArrow({
			...entry(),
			last_return: { method: 'execute', outcome: 'success' },
		});
		useArrowStore.getState().applyRuntimeUpdate({
			namespace:   'github.com/foo/bar@v1',
			state:       'ready',
			active_run:  null,
			last_return: null,
		});
		const e = useArrowStore.getState().arrows.get('github.com/foo/bar@v1')!;
		expect(e.last_return?.outcome).toBe('success');
	});

	it('is a no-op for unknown namespace', () => {
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'unknown@v1', state: 'running', active_run: null, last_return: null,
		});
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});

describe('resetArrows', () => {
	it('clears the map', () => {
		useArrowStore.getState().upsertArrow(entry());
		useArrowStore.getState().resetArrows();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});
