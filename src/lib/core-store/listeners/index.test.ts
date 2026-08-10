import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
}));

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
	coreIsReachable: vi.fn(),
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
	isReconnectSentinel: (d: unknown) =>
		typeof d === 'object' && d !== null && (d as { reconnected?: unknown }).reconnected === true,
}));

vi.mock('../dtos/v0/runtime', async (importOriginal) => {
	const actual = await importOriginal<typeof import('../dtos/v0/runtime')>();
	return { ...actual, toRuntimeUpdate: vi.fn(actual.toRuntimeUpdate) };
});

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch, coreIsReachable } from '@/lib/transport/api';
import { wsManager } from '@/lib/transport/ws-manager';

import { setupListeners } from './index';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

const mockInvoke = invoke as MockedFunction<typeof invoke>;
const mockListen = listen as MockedFunction<typeof listen>;
const mockSubscribeArrowStream = subscribeArrowStream as MockedFunction<typeof subscribeArrowStream>;
const mockGetArrowsFor = getArrowsFor as MockedFunction<typeof getArrowsFor>;
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
const mockCoreIsReachable = coreIsReachable as MockedFunction<typeof coreIsReachable>;
const mockWipe = maybeWipeOnVersionChange as MockedFunction<typeof maybeWipeOnVersionChange>;
const mockWsSubscribe = wsManager.subscribe as MockedFunction<typeof wsManager.subscribe>;
const mockToRuntimeUpdate = toRuntimeUpdate as MockedFunction<typeof toRuntimeUpdate>;

const handlers = new Map<string, (e: { payload: unknown }) => Promise<void> | void>();

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

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((r) => {
		resolve = r;
	});
	return { promise, resolve };
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
		handlers.set(event as string, handler as (e: { payload: unknown }) => Promise<void> | void);
		return Promise.resolve(() => {});
	});
	mockSubscribeArrowStream.mockReturnValue(vi.fn());
	mockGetArrowsFor.mockResolvedValue([]);
	mockApiFetch.mockResolvedValue([]);
	mockWipe.mockResolvedValue(undefined);
	mockCoreIsReachable.mockResolvedValue(false);
	mockInvoke.mockResolvedValue({ connections: [], active_id: 'local' });
	useArrowStore.getState().reset();
	useStatusStore.setState({ status: 'starting' });
});

describe('setupListeners', () => {
	it('registers exactly core://status', async () => {
		await setupListeners();
		const channels = mockListen.mock.calls.map((args) => args[0]);
		expect(channels).toEqual(['core://status']);
	});

	it('registers the listener without waiting for the cache wipe to finish', async () => {
		const order: string[] = [];
		const wipeGate = deferred<void>();
		mockWipe.mockImplementation(() => {
			order.push('wipe:start');
			return wipeGate.promise.then(() => {
				order.push('wipe:done');
			});
		});
		mockListen.mockImplementation((event: unknown, handler: unknown) => {
			order.push(`listen:${event as string}`);
			handlers.set(event as string, handler as (e: { payload: unknown }) => Promise<void> | void);
			return Promise.resolve(() => {});
		});

		const setup = setupListeners();
		await vi.waitFor(() => expect(order).toContain('listen:core://status'));
		wipeGate.resolve();
		await setup;
		await vi.waitFor(() => expect(order).toContain('wipe:done'));

		expect(order).toEqual(['wipe:start', 'listen:core://status', 'wipe:done']);
	});

	it('does not start a stream — and so never seeds the cache — until the wipe has completed', async () => {
		const wipeGate = deferred<void>();
		mockWipe.mockReturnValueOnce(wipeGate.promise);

		const setup = setupListeners();
		await vi.waitFor(() => expect(handlers.has('core://status')).toBe(true));
		const handler = handlers.get('core://status')!;

		const ready = handler({ payload: { status: 'ready' } });
		await vi.waitFor(() => expect(useStatusStore.getState().status).toBe('ready'));
		expect(subscribeArrowStream).not.toHaveBeenCalled();

		wipeGate.resolve();
		await ready;
		await setup;

		expect(subscribeArrowStream).toHaveBeenCalled();
	});

	it('starts the streams anyway when the boot ready was emitted before the listener existed', async () => {
		mockCoreIsReachable.mockResolvedValue(true);
		mockInvoke.mockResolvedValue({ connections: [], active_id: 'local' });

		await setupListeners();

		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'local' }));
		expect(wsManager.subscribe).toHaveBeenCalledWith('/v0/runtime', expect.any(Function));
		expect(useStatusStore.getState().status).toBe('ready');
	});

	it('adopts the connection the Rust side reports active, not a hardcoded local', async () => {
		mockCoreIsReachable.mockResolvedValue(true);
		mockInvoke.mockResolvedValue({ connections: [], active_id: 'remote-7' });
		await setupListeners();
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'remote-7' }));
	});

	it('leaves the boot to the event when the core is not reachable yet', async () => {
		mockCoreIsReachable.mockResolvedValue(false);
		await setupListeners();
		expect(subscribeArrowStream).not.toHaveBeenCalled();
		expect(useStatusStore.getState().status).toBe('starting');
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);
	});

	it('does not restart streams the ready event already started while the probe was in flight', async () => {
		const probe = deferred<boolean>();
		mockCoreIsReachable.mockReturnValueOnce(probe.promise);

		const setup = setupListeners();
		await vi.waitFor(() => expect(handlers.has('core://status')).toBe(true));
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);

		probe.resolve(true);
		await setup;

		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);
	});

	it('does not start a second pair of streams when the probe answers while a ready is still parked mid-start', async () => {
		const connections = deferred<{ connections: never[]; active_id: string }>();
		mockInvoke.mockReturnValueOnce(connections.promise);
		const probe = deferred<boolean>();
		mockCoreIsReachable.mockReturnValueOnce(probe.promise);

		const setup = setupListeners();
		await vi.waitFor(() => expect(handlers.has('core://status')).toBe(true));
		const handler = handlers.get('core://status')!;

		const ready = handler({ payload: { status: 'ready' } });
		await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
		expect(subscribeArrowStream).not.toHaveBeenCalled();

		probe.resolve(true);
		await setup;

		connections.resolve({ connections: [], active_id: 'local' });
		await ready;

		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);
		expect(wsManager.subscribe).toHaveBeenCalledTimes(1);
		expect(invoke).toHaveBeenCalledTimes(1);
	});

	it('does not adopt while a newer start is still parked, after an older superseded one has finished', async () => {
		const gen0 = deferred<{ connections: never[]; active_id: string }>();
		const gen1 = deferred<{ connections: never[]; active_id: string }>();
		mockInvoke.mockReturnValueOnce(gen0.promise).mockReturnValueOnce(gen1.promise);
		const probe = deferred<boolean>();
		mockCoreIsReachable.mockReturnValueOnce(probe.promise);

		const setup = setupListeners();
		const handler = handlers.get('core://status')!;

		const readyG0 = handler({ payload: { status: 'ready' } });
		await handler({ payload: { status: 'starting' } });
		const readyG1 = handler({ payload: { status: 'ready' } });

		await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
		expect(subscribeArrowStream).not.toHaveBeenCalled();

		gen0.resolve({ connections: [], active_id: 'local' });
		await readyG0;

		probe.resolve(true);
		await setup;

		gen1.resolve({ connections: [], active_id: 'local' });
		await readyG1;

		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);
		expect(wsManager.subscribe).toHaveBeenCalledTimes(1);
	});

	it('does not let a failing get_connections on the probe route escape setupListeners', async () => {
		mockCoreIsReachable.mockResolvedValue(true);
		mockInvoke.mockRejectedValueOnce(new Error('socket gone'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(setupListeners()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		expect(subscribeArrowStream).not.toHaveBeenCalled();
		logged.mockRestore();
	});

	it('does not adopt a core that started restarting while the probe was in flight', async () => {
		const probe = deferred<boolean>();
		mockCoreIsReachable.mockReturnValueOnce(probe.promise);

		const setup = setupListeners();
		await vi.waitFor(() => expect(handlers.has('core://status')).toBe(true));
		await emit('core://status', { status: 'starting' });

		probe.resolve(true);
		await setup;

		expect(subscribeArrowStream).not.toHaveBeenCalled();
		expect(useStatusStore.getState().status).toBe('starting');
	});

	it('forwards the status payload to useStatusStore', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(useStatusStore.getState().status).toBe('ready');
	});

	it('starts an arrow stream when core reports ready', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'local' }));
	});

	it('queries get_connections on every ready to learn the active connection', async () => {
		mockInvoke.mockResolvedValue({ connections: [], active_id: 'remote-9' });
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		expect(invoke).toHaveBeenCalledWith('get_connections');
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'remote-9' }));
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

	it("seeds the initial runtime state from the seed's versions[].state, since neither stream pushes on connect", async () => {
		mockApiFetch.mockResolvedValue([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		mockGetArrowsFor.mockResolvedValue([catalogRecord('a@1')]);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];
		await opts.seed();
		opts.onChange?.();
		await vi.waitFor(() => expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running'));
	});

	it('drops a stale onChange read if a starting event lands while getArrowsFor is in flight', async () => {
		const gate = deferred<Array<ReturnType<typeof catalogRecord>>>();
		mockGetArrowsFor.mockReturnValueOnce(gate.promise);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];

		const onChangePromise = opts.onChange?.();
		await emit('core://status', { status: 'starting' });
		gate.resolve([catalogRecord('old-connection-arrow@1')]);
		await onChangePromise;

		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	it("does not discard a newer seed's state for a namespace an earlier, stale onChange read cannot see yet", async () => {
		mockApiFetch.mockResolvedValueOnce([
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
		await opts.seed();

		const gen1Read = deferred<Array<ReturnType<typeof catalogRecord>>>();
		mockGetArrowsFor.mockReturnValueOnce(gen1Read.promise);
		const gen1OnChange = opts.onChange?.();

		mockApiFetch.mockResolvedValueOnce([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'ready' }],
			},
			{
				namespace: 'b',
				name: 'b',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		await opts.seed();

		gen1Read.resolve([catalogRecord('a@1')]);
		await gen1OnChange;

		mockGetArrowsFor.mockResolvedValueOnce([catalogRecord('a@1'), catalogRecord('b@1')]);
		await opts.onChange?.();

		expect(useArrowStore.getState().arrows.get('b@1')?.state).toBe('running');
	});

	it('discards a batch whose own seed bailed, even when the correcting reseed itself fails, rather than misapplying it to a namespace installed later', async () => {
		mockApiFetch.mockResolvedValueOnce([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'ready' }],
			},
			{
				namespace: 'b',
				name: 'b',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];
		await opts.seed();

		mockGetArrowsFor.mockResolvedValueOnce([]);
		await opts.onChange?.();

		mockApiFetch.mockRejectedValueOnce(new Error('network blip'));
		await expect(opts.seed()).rejects.toThrow('network blip');

		mockGetArrowsFor.mockResolvedValueOnce([catalogRecord('b@1')]);
		await opts.onChange?.();

		expect(useArrowStore.getState().arrows.get('b@1')?.state).toBe('absent');
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

	it('restarts the stream against the new connection on a switch, without relying on connection://changed', async () => {
		mockInvoke
			.mockResolvedValueOnce({ connections: [], active_id: 'local' })
			.mockResolvedValueOnce({ connections: [], active_id: 'remote-1' });
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		await emit('core://status', { status: 'starting' });
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenLastCalledWith(expect.objectContaining({ connectionId: 'remote-1' }));
	});

	it('drops a resolved ready if a starting event superseded it while get_connections was in flight', async () => {
		const gate = deferred<{ connections: never[]; active_id: string }>();
		mockInvoke.mockReturnValueOnce(gate.promise);
		await setupListeners();
		const handler = handlers.get('core://status')!;

		const readyPromise = handler({ payload: { status: 'ready' } });
		await handler({ payload: { status: 'starting' } });
		gate.resolve({ connections: [], active_id: 'local' });
		await readyPromise;

		expect(subscribeArrowStream).not.toHaveBeenCalled();
	});

	it('still starts streams for a later, non-superseded ready after a dropped race', async () => {
		const gate = deferred<{ connections: never[]; active_id: string }>();
		mockInvoke.mockReturnValueOnce(gate.promise);
		await setupListeners();
		const handler = handlers.get('core://status')!;

		const readyPromise = handler({ payload: { status: 'ready' } });
		await handler({ payload: { status: 'starting' } });
		gate.resolve({ connections: [], active_id: 'local' });
		await readyPromise;

		mockInvoke.mockResolvedValueOnce({ connections: [], active_id: 'remote-2' });
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'remote-2' }));
	});

	it('applies runtime frames onto the store', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		useArrowStore.getState().setCatalog([catalogRecord('a@1')]);
		const cb = runtimeSubscriber();
		cb({ namespace: 'a@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.get('a@1')?.state).toBe('running');
	});

	it('does not create an entry for a runtime frame naming an unknown namespace', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const cb = runtimeSubscriber();
		cb({ namespace: 'ghost@1', state: 'running', active_run: null, last_return: null });
		expect(useArrowStore.getState().arrows.has('ghost@1')).toBe(false);
	});

	it('ignores the reconnect sentinel on the runtime channel', async () => {
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const cb = runtimeSubscriber();
		cb({ reconnected: true });
		expect(mockToRuntimeUpdate).not.toHaveBeenCalled();
	});
});
