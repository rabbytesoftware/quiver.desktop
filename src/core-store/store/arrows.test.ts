import { describe, it, expect, beforeEach } from 'vitest';

import type { ArrowListItem } from '@/domain/arrow';

import { useArrowStore } from './arrows';

const makeArrow = (ns: string): ArrowListItem => ({
	namespace: ns,
	name: 'X',
	version: '1.0',
	state: 'ready',
	active_run: null,
	last_outcome: null,
});

beforeEach(() => {
	useArrowStore.setState({ arrows: new Map() });
});

describe('useArrowStore', () => {
	it('upsertArrow adds item', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeDefined();
	});

	it('removeArrow deletes item', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().removeArrow('ns@v1');
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeUndefined();
	});

	it('hydrateArrows merges without wiping', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().hydrateArrows([makeArrow('ns@v2')]);
		expect(useArrowStore.getState().arrows.size).toBe(2);
	});

	it('resetArrows clears all entries', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().resetArrows();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it('applyRuntimeUpdate patches existing arrow', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'ns@v1',
			state: 'running',
			active_run: { method: '_execute', variables: {}, steps: [] },
			last_outcome: null,
		});
		expect(useArrowStore.getState().arrows.get('ns@v1')?.state).toBe('running');
	});

	it('applyRuntimeUpdate is no-op for unknown namespace', () => {
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'unknown@v1',
			state: 'running',
			active_run: null,
			last_outcome: null,
		});
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});
});
