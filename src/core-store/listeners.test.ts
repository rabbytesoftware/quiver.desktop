import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { setupListeners } from './listeners';
import { useArrowStore } from './store';
import type { ArrowListItem } from '@/domain/arrow';

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(() => Promise.resolve(() => {})),
}));

const mockListen = listen as MockedFunction<typeof listen>;

const makeArrow = (ns: string): ArrowListItem => ({
    namespace: ns, name: 'X', version: '1.0', state: 'ready', active_run: null, last_outcome: null,
});

beforeEach(() => {
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
    useArrowStore.setState({ arrows: new Map(), status: 'starting' });
});

function captureHandler(eventName: string): (payload: unknown) => void {
    const calls = mockListen.mock.calls;
    const call = calls.find(([name]) => name === eventName);
    if (!call) throw new Error(`No listener registered for ${eventName}`);
    return (payload) => (call[1] as (e: { payload: unknown; event: string; id: number; windowLabel: string }) => void)({ payload, event: eventName, id: 0, windowLabel: '' });
}

describe('setupListeners', () => {
    it('registers handlers for all four events', async () => {
        await setupListeners();
        const registeredEvents = mockListen.mock.calls.map(([name]) => name);
        expect(registeredEvents).toContain('core://status');
        expect(registeredEvents).toContain('arrow://hydrate');
        expect(registeredEvents).toContain('arrow://remove');
        expect(registeredEvents).toContain('runtime://update');
    });

    it('core://status updates store status', async () => {
        await setupListeners();
        captureHandler('core://status')({ status: 'ready' });
        expect(useArrowStore.getState().status).toBe('ready');
    });

    it('arrow://hydrate adds arrows to store', async () => {
        await setupListeners();
        captureHandler('arrow://hydrate')([makeArrow('ns@v1')]);
        expect(useArrowStore.getState().arrows.get('ns@v1')).toBeDefined();
    });

    it('arrow://remove deletes arrow from store', async () => {
        useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
        await setupListeners();
        captureHandler('arrow://remove')({ namespace: 'ns@v1' });
        expect(useArrowStore.getState().arrows.get('ns@v1')).toBeUndefined();
    });

    it('runtime://update applies state change', async () => {
        useArrowStore.getState().upsertArrow(makeArrow('ns@v1'));
        await setupListeners();
        captureHandler('runtime://update')({
            namespace: 'ns@v1',
            state: 'running',
            active_run: { method: '_execute', variables: {}, steps: [] },
            last_outcome: null,
        });
        expect(useArrowStore.getState().arrows.get('ns@v1')?.state).toBe('running');
    });
});
