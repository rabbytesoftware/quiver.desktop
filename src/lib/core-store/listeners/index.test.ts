import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(),
}));

vi.mock('@/lib/persistence/idb', () => ({
	maybeWipeOnVersionChange: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/lib/persistence/entity-stream', () => ({
	subscribeArrowStream: vi.fn(),
}));

vi.mock('@/lib/persistence/entity-cache', () => ({
	getArrowsFor: vi.fn(),
}));

vi.mock('@/lib/transport/api', () => ({
	apiFetch: vi.fn(),
}));

const runtimeSubscribers: Array<(d: unknown) => void> = [];
vi.mock('@/lib/transport/ws-manager', () => ({
	wsManager: {
		subscribe: vi.fn((_endpoint: string, cb: (d: unknown) => void) => {
			runtimeSubscribers.push(cb);
			return vi.fn();
		}),
		send: vi.fn(),
	},
	// Real predicate, not a stub — a test asserting sentinel-handling would not
	// discriminate a broken predicate from a broken call site otherwise.
	isReconnectSentinel: (d: unknown) =>
		typeof d === 'object' && d !== null && (d as { reconnected?: unknown }).reconnected === true,
}));

vi.mock('../dtos/v0/runtime', async (importOriginal) => {
	const actual = await importOriginal<typeof import('../dtos/v0/runtime')>();
	return { ...actual, toRuntimeUpdate: vi.fn(actual.toRuntimeUpdate) };
});

import { listen } from '@tauri-apps/api/event';

import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch } from '@/lib/transport/api';
import { wsManager } from '@/lib/transport/ws-manager';

import { setupListeners } from './index';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

const mockListen = listen as MockedFunction<typeof listen>;
const mockSubscribeArrowStream = subscribeArrowStream as MockedFunction<typeof subscribeArrowStream>;
const mockGetArrowsFor = getArrowsFor as MockedFunction<typeof getArrowsFor>;
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
const mockWsSubscribe = wsManager.subscribe as MockedFunction<typeof wsManager.subscribe>;
const mockToRuntimeUpdate = toRuntimeUpdate as MockedFunction<typeof toRuntimeUpdate>;

const handlers = new Map<string, (e: { payload: unknown }) => void>();

async function emit(event: string, payload: unknown): Promise<void> {
	const handler = handlers.get(event);
	if (!handler) throw new Error(`no handler registered for ${event}`);
	await handler({ payload });
}

function runtimeSubscriber(): (d: unknown) => void {
	const last = runtimeSubscribers[runtimeSubscribers.length - 1];
	if (!last) throw new Error('runtime endpoint was never subscribed');
	return last;
}

const catalogRecord = (namespace: string) => ({
	connectionId: 'local',
	namespace,
	name: namespace,
	description: '',
	tags: [],
	icon: null,
	banner: null,
	version: '1',
});

beforeEach(() => {
	vi.clearAllMocks();
	handlers.clear();
	runtimeSubscribers.length = 0;
	mockListen.mockImplementation((event: unknown, handler: unknown) => {
		handlers.set(event as string, handler as (e: { payload: unknown }) => void);
		return Promise.resolve(() => {});
	});
	mockSubscribeArrowStream.mockReturnValue(vi.fn());
	mockGetArrowsFor.mockResolvedValue([]);
	mockApiFetch.mockResolvedValue([]);
	useArrowStore.getState().reset();
	useStatusStore.setState({ status: 'starting' });
});

describe('setupListeners', () => {
	it('registers exactly core://status and connection://changed', async () => {
		await setupListeners();
		const channels = mockListen.mock.calls.map((args) => args[0]);
		expect(channels).toEqual(['core://status', 'connection://changed']);
	});

	it('wipes a stale cache before registering any listener', async () => {
		const order: string[] = [];
		(maybeWipeOnVersionChange as MockedFunction<typeof maybeWipeOnVersionChange>).mockImplementation(() => {
			order.push('wipe');
			return Promise.resolve();
		});
		mockListen.mockImplementation((event: unknown, handler: unknown) => {
			order.push(`listen:${event as string}`);
			handlers.set(event as string, handler as (e: { payload: unknown }) => void);
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(order).toEqual(['wipe', 'listen:core://status', 'listen:connection://changed']);
	});

	it('forwards the status payload to useStatusStore', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(useStatusStore.getState().status).toBe('ready');
	});

	// core://status stays a Tauri event: sidecar readiness is a native fact the
	// webview cannot observe for itself.
	it('starts an arrow stream when core reports ready', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'local' }));
	});

	it('also subscribes the runtime overlay endpoint when core reports ready', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(wsManager.subscribe).toHaveBeenCalledWith('/v0/runtime', expect.any(Function));
	});

	it("the arrow stream's seed GETs the user-installed catalog and stamps the active connection", async () => {
		mockApiFetch.mockResolvedValue([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'ready' }],
			},
		]);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];
		const records = await opts.seed();
		expect(apiFetch).toHaveBeenCalledWith('/v0/arrow?user_installed=true');
		expect(records).toEqual([
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
	});

	it("the arrow stream's onChange pushes the cached rows for the active connection into the catalog", async () => {
		mockGetArrowsFor.mockResolvedValue([catalogRecord('x@1')]);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];
		opts.onChange?.();
		await vi.waitFor(() => expect(useArrowStore.getState().arrows.get('x@1')).toBeDefined());
		expect(getArrowsFor).toHaveBeenCalledWith('local');
	});

	it('clears the projection and stops the arrow stream when core restarts', async () => {
		const dispose = vi.fn();
		mockSubscribeArrowStream.mockReturnValue(dispose);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		await emit('core://status', { status: 'starting' });
		expect(dispose).toHaveBeenCalled();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it('also stops the runtime overlay subscription when core restarts', async () => {
		const disposeRuntime = vi.fn();
		mockWsSubscribe.mockReturnValueOnce(disposeRuntime);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		await emit('core://status', { status: 'starting' });
		expect(disposeRuntime).toHaveBeenCalled();
	});

	it('a starting event before any ready is a safe no-op beyond resetting the store', async () => {
		await setupListeners();
		await expect(emit('core://status', { status: 'starting' })).resolves.toBeUndefined();
		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it('a disconnected status only updates the status store', async () => {
		await setupListeners();
		await emit('core://status', { status: 'disconnected' });
		expect(useStatusStore.getState().status).toBe('disconnected');
		expect(subscribeArrowStream).not.toHaveBeenCalled();
	});

	it('connection://changed alone (no ready yet) does not start streams', async () => {
		await setupListeners();
		await emit('connection://changed', { connections: [], active_id: 'remote-1' });
		expect(subscribeArrowStream).not.toHaveBeenCalled();
	});

	it('restarts the stream against the new connection on a switch', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		await emit('connection://changed', { connections: [], active_id: 'remote-1' });
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenLastCalledWith(expect.objectContaining({ connectionId: 'remote-1' }));
	});

	it('applies runtime frames onto the store', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		const cb = runtimeSubscriber();
		cb({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	// Trap: the overlay stream must never create or prune a row. This exercises
	// the FULL wiring (real store, real DTO conversion, real listeners logic),
	// not just the store's own unit test.
	it('does not create an entry for a runtime frame naming an unknown namespace', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const cb = runtimeSubscriber();
		cb({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.has('ghost@1')).toBe(false);
	});

	// The reconnect sentinel is not a DTO. Unlike the entity stream, the
	// overlay never reseeds on it — there is nothing to prune or resurrect, so
	// it must never even reach `toRuntimeUpdate`.
	it('ignores the reconnect sentinel on the runtime channel', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const cb = runtimeSubscriber();
		cb({ reconnected: true });
		expect(mockToRuntimeUpdate).not.toHaveBeenCalled();
	});
});
