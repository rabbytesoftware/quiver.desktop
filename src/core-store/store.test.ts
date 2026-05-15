import { describe, it, expect, beforeEach } from 'vitest';

import type { ArrowListItem } from '@/domain';

import { useArrowStore } from './store/arrows';
import { useStatusStore } from './store/status';

const makeArrow = (namespace: string): ArrowListItem => ({
	namespace,
	name: 'Arrow',
	version: '1.0.0',
	state: 'ready',
	active_run: null,
	last_outcome: null,
});

beforeEach(() => {
	useArrowStore.setState({ arrows: new Map() });
	useStatusStore.setState({ status: 'starting' });
});

describe('upsertArrow', () => {
	it('adds a new arrow to the map', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeDefined();
	});

	it('updates an existing arrow without affecting others', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().upsertArrow(makeArrow('ns@v2'));
		useArrowStore.getState().upsertArrow({ ...makeArrow('ns@v1'), name: 'Updated' });
		expect(useArrowStore.getState().arrows.get('ns@v1')?.name).toBe('Updated');
		expect(useArrowStore.getState().arrows.get('ns@v2')).toBeDefined();
	});
});

describe('removeArrow', () => {
	it('removes an arrow by namespace', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().removeArrow('ns@v1');
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeUndefined();
	});
});

describe('hydrateArrows', () => {
	it('merges incoming items without wiping unrelated entries', () => {
		useArrowStore.getState().upsertArrow(makeArrow('existing@v1'));
		useArrowStore.getState().hydrateArrows([makeArrow('new@v1')]);
		expect(useArrowStore.getState().arrows.get('existing@v1')).toBeDefined();
		expect(useArrowStore.getState().arrows.get('new@v1')).toBeDefined();
	});
});

describe('applyRuntimeUpdate', () => {
	it('updates state and active_run for a known namespace', () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'ns@v1',
			state: 'installing',
			active_run: { method: '_install', variables: {}, steps: [] },
			last_outcome: null,
		});
		const arrow = useArrowStore.getState().arrows.get('ns@v1')!;
		expect(arrow.state).toBe('installing');
		expect(arrow.active_run?.method).toBe('_install');
	});

	it('is a no-op for unknown namespaces', () => {
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'unknown@v1',
			state: 'running',
			active_run: null,
			last_outcome: null,
		});
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it('preserves existing last_outcome when update sends null', () => {
		useArrowStore.getState().upsertArrow({
			...makeArrow('ns@v1'),
			last_outcome: { method: '_install', outcome: 'success' },
		});
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: 'ns@v1',
			state: 'ready',
			active_run: null,
			last_outcome: null,
		});
		expect(useArrowStore.getState().arrows.get('ns@v1')?.last_outcome?.method).toBe('_install');
	});
});

describe('setStatus', () => {
	it('updates core status', () => {
		useStatusStore.getState().setStatus('ready');
		expect(useStatusStore.getState().status).toBe('ready');
	});
});
