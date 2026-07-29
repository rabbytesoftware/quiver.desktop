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
	// The boot probe (review finding C1) defaults to "core is not up yet", which
	// is what every pre-existing test here assumed: streams start from the
	// `core://status` event and from nothing else. The probe's own tests opt in.
	mockCoreIsReachable.mockResolvedValue(false);
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

	// Review finding C1: the wipe used to be AWAITED ahead of `listen()`, which
	// put a full IndexedDB open + clear() between app start and the only thing
	// that can catch `core://status`. Tauri drops an event nobody is listening
	// for, so every millisecond spent before registration is a millisecond in
	// which the boot `ready` can be lost for good. The wipe is still started
	// first — it must, so it can be complete before any seed — but it is no
	// longer allowed to delay registration.
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
		// Registered while the wipe is still in flight — that is the whole point.
		await vi.waitFor(() => expect(order).toContain('listen:core://status'));
		wipeGate.resolve();
		await setup;
		await vi.waitFor(() => expect(order).toContain('wipe:done'));

		expect(order).toEqual(['wipe:start', 'listen:core://status', 'wipe:done']);
	});

	// The other half of the same change: moving the wipe off the registration
	// path must NOT weaken design decision #4 — the wipe still has to complete
	// before anything can seed the cache. Nothing else in this suite would
	// notice if `startStreams` stopped waiting for it.
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

	// ── Review finding C1: the dropped boot handshake ────────────────────────
	//
	// `emit_core_status` is fire-and-forget and Tauri buffers nothing, so a
	// `ready` emitted before `listen()` finishes registering is gone. Rust emits
	// it on its FIRST successful `/v0/health`, with no initial delay, so against
	// an already-running daemon that happens milliseconds after setup(). Nothing
	// recovers: `switch_connection` emits no `connection://changed`, and the
	// status store's 'starting' default makes a missed `ready` look exactly like
	// a daemon that never came up. Every other test in this file emits AFTER
	// registration by construction and so cannot see this.
	it('starts the streams anyway when the boot ready was emitted before the listener existed', async () => {
		mockCoreIsReachable.mockResolvedValue(true);
		mockInvoke.mockResolvedValue({ connections: [], active_id: 'local' });

		// No emit at all: the event was dropped and is never coming.
		await setupListeners();

		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'local' }));
		expect(wsManager.subscribe).toHaveBeenCalledWith('/v0/runtime', expect.any(Function));
		// And the UI must stop claiming the core is still starting.
		expect(useStatusStore.getState().status).toBe('ready');
	});

	it('adopts the connection the Rust side reports active, not a hardcoded local', async () => {
		mockCoreIsReachable.mockResolvedValue(true);
		mockInvoke.mockResolvedValue({ connections: [], active_id: 'remote-7' });
		await setupListeners();
		expect(subscribeArrowStream).toHaveBeenCalledWith(expect.objectContaining({ connectionId: 'remote-7' }));
	});

	// The ordinary cold boot: the daemon genuinely is not up yet. The probe must
	// stay out of the way and leave the `ready` event to do its job — starting
	// streams against a daemon that cannot answer would only churn failed GETs.
	it('leaves the boot to the event when the core is not reachable yet', async () => {
		mockCoreIsReachable.mockResolvedValue(false);
		await setupListeners();
		expect(subscribeArrowStream).not.toHaveBeenCalled();
		expect(useStatusStore.getState().status).toBe('starting');
		await emit('core://status', { status: 'ready' });
		expect(subscribeArrowStream).toHaveBeenCalledTimes(1);
	});

	// The probe is async, so a `ready` can arrive and start the streams while it
	// is still in flight. Adopting on top of that would tear down a healthy pair
	// of subscriptions and rebuild them — a needless WS reconnect and a
	// duplicate catalog GET on every launch that wins the race.
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

	// A `starting` seen during the probe means the event channel is alive and a
	// `ready` for the NEW connection is on its way. Adopting here would start
	// streams for a connection that is mid-restart, against whatever `active_id`
	// happened to be current.
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

	// Fix round 3, R1: the generation counter added in round 2 guards the
	// `ready` handler's own `get_connections` await, but a stream's ongoing
	// `onChange` read is a SEPARATE, later async gap the counter didn't reach
	// — the reviewer probed exactly this. Uses the same raw-handler,
	// un-awaited, deferred-gate technique as round 2's ready/starting race
	// test: `emit` always awaits a handler and cannot observe this.
	it('drops a stale onChange read if a starting event lands while getArrowsFor is in flight', async () => {
		const gate = deferred<Array<ReturnType<typeof catalogRecord>>>();
		mockGetArrowsFor.mockReturnValueOnce(gate.promise);
		await setupListeners();
		await emit('core://status', { status: 'ready' });
		const opts = mockSubscribeArrowStream.mock.calls[0][0];

		// Trigger onChange — its getArrowsFor() call is now pending on the gate.
		const onChangePromise = opts.onChange?.();
		// Interleave `starting` before the read resolves — this resets the store.
		await emit('core://status', { status: 'starting' });
		// Now let the stale read resolve, carrying the OLD connection's rows.
		gate.resolve([catalogRecord('old-connection-arrow@1')]);
		await onChangePromise;

		expect(useArrowStore.getState().arrows.size).toBe(0);
	});

	// Originally fix round 3, R2 — re-run here per fix round 4's instruction
	// to confirm the round-4 identity-based redesign has NOT reopened it: if
	// a reseed's GET resolves before an EARLIER onChange's own cache read
	// completes, that earlier, stale-snapshot onChange must not consume the
	// NEWER seed's batch against its OLD catalog view and silently discard
	// the state for a namespace only the newer catalog knows about (here,
	// b@1). Under the round-4 design this holds because gen 1's `onChange`
	// captured ITS OWN batch (gen 1's) by reference before gen 2's seed()
	// ever ran, so it can only ever act on gen 1's data — it never sees, and
	// so never discards, gen 2's.
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
		await opts.seed(); // pendingInitialStates now holds gen 1's [a@1: ready]

		// gen 1's onChange begins; its own cache read is gated and, once it
		// resolves, will reflect gen 1's catalog — a@1 only, no b@1 yet.
		const gen1Read = deferred<Array<ReturnType<typeof catalogRecord>>>();
		mockGetArrowsFor.mockReturnValueOnce(gen1Read.promise);
		const gen1OnChange = opts.onChange?.();

		// gen 2 (a reconnect reseed) resolves its OWN seed() BEFORE gen 1's
		// read above ever settles, overwriting pendingInitialStates with the
		// fresher full set — which now also includes b@1, a namespace gen 1
		// never saw.
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

		// NOW let gen 1's stale read resolve — it can only see a@1.
		gen1Read.resolve([catalogRecord('a@1')]);
		await gen1OnChange;

		// gen 2's own onChange finally runs, with a fresh read that DOES
		// include b@1 — its seeded state must still be applicable here, not
		// already thrown away by gen 1's earlier, stale consumption.
		mockGetArrowsFor.mockResolvedValueOnce([catalogRecord('a@1'), catalogRecord('b@1')]);
		await opts.onChange?.();

		expect(useArrowStore.getState().arrows.get('b@1')?.state).toBe('running');
	});

	// Fix round 4, R2: round 3's "retain what's not yet visible" design had
	// no bound at all — a batch whose OWN seed bailed (nothing ever visible
	// to apply against) survived indefinitely, waiting to be misapplied the
	// moment ANY later onChange happened to reveal a matching namespace, for
	// a completely unrelated reason. Reproduces the reviewer's exact 5-step
	// scenario with real entity-stream/store semantics simulated through the
	// mocked seed()/onChange() call sites this module actually exposes:
	//  1. a reconnect sentinel lands mid-GET (simulated: nothing further —
	//     the effect is what matters, not the WS trigger itself);
	//  2. GET #1 resolves; the seed() closure sets its batch;
	//  3. that seed's OWN applySeed bails (simulated directly: its onChange
	//     fires against a COLD cache, i.e. nothing was ever written);
	//  4. the correcting reseed (GET #2) fails outright — seed() rejects;
	//  5. much later, one of the batch's namespaces (b@1) is installed for
	//     the FIRST time via an ordinary live upsert — an onChange fires
	//     with b@1 now genuinely present for an entirely unrelated reason.
	// b@1 must be neutral ('absent'), not resurrect the abandoned batch's
	// stale 'running' claim about a process that was never actually started.
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
		await opts.seed(); // GET #1 resolves; batch = [a@1: ready, b@1: running]

		// GET #1's own onChange fires against a cold cache: its applySeed
		// bailed (superseded by a reconnect sentinel that landed while GET #1
		// was in flight), so nothing was ever written for it — entity-stream
		// still calls onChange even for a bailed seed.
		mockGetArrowsFor.mockResolvedValueOnce([]);
		await opts.onChange?.();

		// The reseed meant to correct the superseded seed fails outright — a
		// network blip. entity-stream never calls onChange for a thrown seed.
		mockApiFetch.mockRejectedValueOnce(new Error('network blip'));
		await expect(opts.seed()).rejects.toThrow('network blip');

		// Much later, b@1 is installed for the FIRST time via an ordinary
		// live catalog frame — completely unrelated to the abandoned batch.
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
