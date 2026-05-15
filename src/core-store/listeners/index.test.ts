import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { listen } from '@tauri-apps/api/event';

import type { ArrowListItem } from '@/domain/arrow';

import { setupListeners } from './index';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(() => Promise.resolve(() => {})),
}));

const mockListen = listen as MockedFunction<typeof listen>;

const makeArrow = (ns: string): ArrowListItem => ({
	namespace: ns,
	name: 'X',
	version: '1.0',
	state: 'ready',
	active_run: null,
	last_outcome: null,
});

beforeEach(() => {
	mockListen.mockReset();
	mockListen.mockResolvedValue(() => {});
	useArrowStore.setState({ arrows: new Map() });
	useStatusStore.setState({ status: 'starting' });
});

function captureHandler(eventName: string): (payload: unknown) => void {
	const call = mockListen.mock.calls.find(([name]) => name === eventName);
	if (!call) throw new Error(`No listener for ${eventName}`);
	return (payload) =>
		(call[1] as (e: { payload: unknown; event: string; id: number; windowLabel: string }) => void)({
			payload,
			event: eventName,
			id: 0,
			windowLabel: '',
		});
}

describe('setupListeners', () => {
	it('registers handlers for all four events', async () => {
		await setupListeners();
		const events = mockListen.mock.calls.map(([name]) => name);
		expect(events).toContain('core://status');
		expect(events).toContain('arrow://hydrate');
		expect(events).toContain('arrow://event');
		expect(events).toContain('runtime://update');
	});

	it('core://status updates status store', async () => {
		await setupListeners();
		captureHandler('core://status')({ status: 'ready' });
		expect(useStatusStore.getState().status).toBe('ready');
	});

	it('core://status starting resets arrow store', async () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		await setupListeners();
		captureHandler('core://status')({ status: 'starting' });
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it('arrow://hydrate adds arrows', async () => {
		await setupListeners();
		captureHandler('arrow://hydrate')([makeArrow('ns@v1')]);
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeDefined();
	});

	it('arrow://event removed deletes arrow', async () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		await setupListeners();
		captureHandler('arrow://event')({ event: 'removed', namespace: 'ns@v1' });
		expect(useArrowStore.getState().arrows.get('ns@v1')).toBeUndefined();
	});

	it('arrow://event upserted adds new arrow with default state', async () => {
		await setupListeners();
		captureHandler('arrow://event')({ event: 'upserted', namespace: 'ns@v2', name: 'MyArrow', version: '2.0' });
		const arrow = useArrowStore.getState().arrows.get('ns@v2');
		expect(arrow).toBeDefined();
		expect(arrow?.name).toBe('MyArrow');
		expect(arrow?.state).toBe('ready');
		expect(arrow?.active_run).toBeNull();
		expect(arrow?.last_outcome).toBeNull();
	});

	it('arrow://event upserted preserves existing state', async () => {
		useArrowStore.getState().upsertArrow({ ...makeArrow('ns@v1'), state: 'running' });
		await setupListeners();
		captureHandler('arrow://event')({ event: 'upserted', namespace: 'ns@v1', name: 'X', version: '1.1' });
		expect(useArrowStore.getState().arrows.get('ns@v1')?.state).toBe('running');
		expect(useArrowStore.getState().arrows.get('ns@v1')?.version).toBe('1.1');
	});

	it('runtime://update applies state change', async () => {
		useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
		await setupListeners();
		captureHandler('runtime://update')({
			namespace: 'ns@v1',
			state: 'running',
			active_run: { method: '_execute', variables: {}, steps: [] },
			last_return: null,
		});
		expect(useArrowStore.getState().arrows.get('ns@v1')?.state).toBe('running');
	});
});
