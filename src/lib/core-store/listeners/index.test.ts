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

import { invoke } from '@tauri-apps/api/core';
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

const mockInvoke = invoke as MockedFunction<typeof invoke>;
const mockListen = listen as MockedFunction<typeof listen>;
const mockSubscribeArrowStream = subscribeArrowStream as MockedFunction<typeof subscribeArrowStream>;
const mockGetArrowsFor = getArrowsFor as MockedFunction<typeof getArrowsFor>;
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
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
	// Every `ready` self-sufficiently queries the active connection — see
	// review finding 2 (fix round 1). Default to the same connection every
	// existing test already assumed before that fix.
	mockInvoke.mockResolvedValue({ connections: [], active_id: 'local' });
	useArrowStore.getState().reset();
	useStatusStore.setState({ status: 'starting' });
});

describe('setupListeners', () => {
	// connection://changed still exists (add/remove/rename emit it) but is
	// consumed by @/lib/connection/listeners.ts, not here — see fix round 2's
	// Minor: this module registered a no-op handler for it that only the OLD
	// version of this test kept "alive". Registering nothing for it is the
	// honest state of what this module now depends on.
	it('registers exactly core://status', async () => {
		await setupListeners();
		const channels = mockListen.mock.calls.map((args) => args[0]);
		expect(channels).toEqual(['core://status']);
	});

	it('wipes a stale cache before registering any listener', async () => {
		const order: string[] = [];
		(maybeWipeOnVersionChange as MockedFunction<typeof maybeWipeOnVersionChange>).mockImplementation(() => {
			order.push('wipe');
			return Promise.resolve();
		});
		mockListen.mockImplementation((event: unknown, handler: unknown) => {
			order.push(`listen:${event as string}`);
			handlers.set(event as string, handler as (e: { payload: unknown }) => Promise<void> | void);
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(order).toEqual(['wipe', 'listen:core://status']);
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

	// Review finding 2 (fix round 1): `switch_connection` does not emit
	// `connection://changed` at all (a Rust-side gap), so `ready` cannot trust
	// that event — it must ask `get_connections` directly, every time.
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

	// Review finding 5 (fix round 1): neither /v0/arrow nor /v0/runtime push
	// anything on connect (verified against stable-26.5.1) — without this, an
	// already-running arrow would render as 'absent' forever, since no
	// transition frame is ever coming to correct it.
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

	// Review finding 2 (fix round 1): reproduces the REAL production sequence
	// for a switch — ready -> starting -> ready, driven entirely by
	// get_connections' active_id changing between calls, with NO
	// connection://changed at all (which switch_connection does not emit).
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

	// Review finding 2 (fix round 2): Tauri does NOT serialize async event
	// handlers, so a `starting` can land while an earlier `ready`'s own
	// get_connections() call is still in flight. The `emit` helper above
	// always awaits a handler before returning, which masks exactly this —
	// so this test drives the two handler calls directly and interleaves them
	// itself, without awaiting the first, to actually observe the race.
	it('drops a resolved ready if a starting event superseded it while get_connections was in flight', async () => {
		const gate = deferred<{ connections: never[]; active_id: string }>();
		mockInvoke.mockReturnValueOnce(gate.promise);
		await setupListeners();
		const handler = handlers.get('core://status')!;

		// Fire `ready` but do NOT await it — its get_connections() call is
		// still pending.
		const readyPromise = handler({ payload: { status: 'ready' } });
		// Interleave `starting` before the ready's invoke ever settles.
		await handler({ payload: { status: 'starting' } });
		// Now let the superseded ready's invoke resolve.
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
