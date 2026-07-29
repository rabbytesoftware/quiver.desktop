import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const subscribers: Array<(d: unknown) => void> = [];
vi.mock('@/lib/transport/ws-manager', () => ({
	wsManager: {
		subscribe: (_e: string, cb: (d: unknown) => void) => {
			subscribers.push(cb);
			return () => {};
		},
		send: () => {},
	},
	isReconnectSentinel: (d: unknown) => (d as { reconnected?: boolean })?.reconnected === true,
}));

const { subscribeArrowStream } = await import('./entity-stream');
const { getArrowsFor } = await import('./entity-cache');
const { resetDB } = await import('./idb');

const rec = (namespace: string) => ({
	connectionId: 'local',
	namespace,
	name: namespace,
	description: '',
	tags: [],
	icon: null,
	banner: null,
	version: '1',
});

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((r) => {
		resolve = r;
	});
	return { promise, resolve };
}

describe('entity-stream', () => {
	beforeEach(() => {
		subscribers.length = 0;
		// fake-indexeddb persists its backing store across `it()`s within the
		// same file; resetDB() alone only forgets idb.ts's cached connection
		// handle, not the underlying data. A fresh in-memory factory per test
		// gives real isolation (the standard fake-indexeddb reset idiom).
		globalThis.indexedDB = new IDBFactory();
		resetDB();
	});

	// A spy left in place after a failing assertion would otherwise leak into
	// later tests — `restoreMocks` isn't set in vitest.config.ts, so this has
	// to be explicit rather than relying on inline `spy.mockRestore()` calls
	// that a thrown assertion would skip.
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('seeds the cache before any frame arrives', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [rec('a@1')], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		// Full content, not just a row count: a seed that upserted a stub record
		// (e.g. only `{ connectionId, namespace }`) would still pass a length
		// check but silently drop every other catalog field.
		expect(await getArrowsFor('local')).toEqual([rec('a@1')]);
	});

	// A GET is a point-in-time snapshot. An arrow deleted while the app was
	// closed would otherwise linger forever, because no frame will ever
	// announce it.
	it('prunes cached rows the seed no longer lists', async () => {
		const first = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [rec('a@1'), rec('b@1')], onChange: first });
		await vi.waitFor(() => expect(first).toHaveBeenCalled());

		const second = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [rec('a@1')], onChange: second });
		await vi.waitFor(() => expect(second).toHaveBeenCalled());
		expect((await getArrowsFor('local')).map((r) => r.namespace)).toEqual(['a@1']);
	});

	it('never prunes another connection rows', async () => {
		const other = vi.fn();
		subscribeArrowStream({
			connectionId: 'remote-1',
			seed: async () => [rec('z@1')].map((r) => ({ ...r, connectionId: 'remote-1' })),
			onChange: other,
		});
		await vi.waitFor(() => expect(other).toHaveBeenCalled());

		const mine = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: mine });
		await vi.waitFor(() => expect(mine).toHaveBeenCalled());
		expect(await getArrowsFor('remote-1')).toHaveLength(1);
	});

	it('merges a live upsert frame', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({
			event: 'upserted',
			namespace: 'new@1',
			name: 'New Arrow',
			description: 'a description',
			tags: ['a', 'b'],
			icon: 'icon.png',
			banner: 'banner.png',
			version: '2',
		});
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
		// Exact keys, not `toMatchObject`: catches both a dropped catalog field
		// AND a leaked transport field. `{ ...frame, connectionId }` would spread
		// `event: 'upserted'` in alongside the 7 real catalog fields — a 9-key
		// record `toEqual` would reject, since it fails on extra properties too.
		expect(await getArrowsFor('local')).toEqual([
			{
				connectionId: 'local',
				namespace: 'new@1',
				name: 'New Arrow',
				description: 'a description',
				tags: ['a', 'b'],
				icon: 'icon.png',
				banner: 'banner.png',
				version: '2',
			},
		]);
	});

	// Only `event` and `namespace` are wire-guaranteed on an 'upserted' frame
	// (per the ArrowFrame doc comment); every other catalog field is optional
	// and must default rather than persist `undefined`.
	it('defaults the optional catalog fields on a minimal upsert frame', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({ event: 'upserted', namespace: 'bare@1' });
		await vi.waitFor(async () =>
			expect(await getArrowsFor('local')).toEqual([
				{
					connectionId: 'local',
					namespace: 'bare@1',
					name: '',
					description: '',
					tags: [],
					icon: null,
					banner: null,
					version: '',
				},
			])
		);
	});

	// The H21 race: an upsert and a delete for one namespace, each on its own
	// fire-and-forget IDB transaction, can commit out of order — and a late
	// upsert resurrects a tombstoned row. The serial chain is what forbids it.
	//
	// Under a fast fake-indexeddb both operations settle in the same microtask
	// tick regardless of chaining, so a plain dispatch never actually exposes
	// the race (verified by mutation: removing the chain left this test green).
	// A prior version of this test used a fixed setTimeout delay plus a fixed
	// wall-clock wait before asserting; the margin between them was too tight
	// (a 20ms delay racing a 40ms wait, both subject to real scheduler jitter)
	// and broke in both directions under load.
	//
	// A SECOND prior version gated the upsert's own IDB write on a `deferred()`
	// and then did `await upsertSettled`, where `upsertSettled` was a `let`
	// the mock reassigned when invoked. That is subtly wrong: `await x` reads
	// `x` ONCE, synchronously, before suspending — it does not re-read the
	// binding later. Under the real (chained) implementation, `applyFrame` for
	// the upsert frame is only invoked on a LATER microtask (queued via
	// `applyChain.then(...)`), so at the moment the `await` evaluated `x` it
	// was still the stale initial `Promise.resolve()` — an accidental no-op
	// wait that only happened to pass because the subsequent `vi.waitFor` poll
	// loop had enough real retries to catch up regardless. Under the mutant
	// (fire-and-forget `applyFrame` call, invoked synchronously) that same
	// staleness meant the test's very first `vi.waitFor` poll could land in the
	// narrow window AFTER the delete resolved but BEFORE the resurrecting
	// upsert did — and pass on an empty cache it never should have observed.
	// Confirmed both effects by chronological instrumentation before fixing.
	//
	// This version doesn't read anything the mock produces until AFTER
	// confirming the mock was actually invoked. `invoked` is a plain signal
	// (deferred<void>, no payload — resolving a deferred WITH a promise as its
	// value would itself auto-flatten per the Promise spec, reintroducing a
	// different version of the same trap) that the mock resolves the moment it
	// runs, before writing the real completion promise into `ref.settled`.
	// `ref.settled` is only read AFTER `await invoked.promise`, by which point
	// the write is guaranteed to have happened — unlike a `let` captured by
	// the test body before the mock ever ran.
	it('commits an upsert then a delete in arrival order', async () => {
		const cacheMod = await import('./entity-cache');
		const realUpsertArrow = cacheMod.upsertArrow;
		const gate = deferred<void>();
		const invoked = deferred<void>();
		const ref: { settled: Promise<void> } = { settled: Promise.resolve() };
		vi.spyOn(cacheMod, 'upsertArrow').mockImplementationOnce((r) => {
			ref.settled = gate.promise.then(() => realUpsertArrow(r));
			invoked.resolve();
			return ref.settled;
		});
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({ event: 'upserted', namespace: 'x@1', name: 'x', description: '', tags: [] });
		subscribers[0]({ event: 'removed', namespace: 'x@1' });
		gate.resolve();
		await invoked.promise;
		await ref.settled;
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toEqual([]));
	});

	it('reseeds on the reconnect sentinel', async () => {
		const seed = vi.fn().mockResolvedValue([]);
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed, onChange: done });
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(1));
		subscribers[0]({ reconnected: true });
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(2));
	});

	// A .then on a rejected promise skips every later step. Without the catch a
	// single failed seed freezes the stream for the whole session: no frames,
	// and no reconnect reseed could ever recover it.
	it('survives a failed seed and still applies later frames', async () => {
		const seed = vi.fn().mockRejectedValueOnce(new Error('down')).mockResolvedValue([]);
		subscribeArrowStream({ connectionId: 'local', seed });
		await vi.waitFor(() => expect(seed).toHaveBeenCalled());
		subscribers[0]({ event: 'upserted', namespace: 'after@1', name: 'after', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	// The generation counter's job is stronger than "seed() gets called again":
	// a reseed that is still IN FLIGHT when a newer reconnect fires must write
	// NOTHING, not merely get quietly cleaned up by the newer reseed's own
	// prune. Generation 2's own GET is held open here so nothing generation-2
	// does can be responsible for the assertion — only the guard can be.
	it('a seed superseded by a newer reconnect writes nothing', async () => {
		const gen1 = deferred<ReturnType<typeof rec>[]>();
		const gen2 = deferred<ReturnType<typeof rec>[]>();
		const seed = vi.fn().mockReturnValueOnce(gen1.promise).mockReturnValueOnce(gen2.promise);
		subscribeArrowStream({ connectionId: 'local', seed });
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(1));

		// Bumps seedGeneration to 2 immediately, even though generation 2's own
		// applySeed step is queued strictly after generation 1 on the serial
		// chain and has not run yet.
		subscribers[0]({ reconnected: true });

		gen1.resolve([rec('a@1')]);
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(2));
		// Give the (superseded) generation-1 step a chance to run its write path
		// if the guard did not stop it; generation 2 is still blocked on its own
		// GET, so it cannot be what keeps the cache empty here.
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);

		gen2.resolve([rec('b@1')]);
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	// The guard is checked TWICE in applySeed — once after the GET, once after
	// the getArrowsFor() diff read — because a supersession can land in either
	// gap. This drives one in specifically during the second gap: getArrowsFor
	// itself is the trigger, so the first check has already passed by the time
	// it fires.
	it('a seed superseded between its GET and its diff read still writes nothing', async () => {
		const cacheMod = await import('./entity-cache');
		const realGetArrowsFor = cacheMod.getArrowsFor;
		vi.spyOn(cacheMod, 'getArrowsFor').mockImplementationOnce(async (id: string) => {
			subscribers[0]({ reconnected: true });
			return realGetArrowsFor(id);
		});
		// Generation 2's own GET is held open, exactly as in the test above, so
		// generation 2 cannot reach its own diff/prune before the assertion below
		// runs — it is stuck awaiting its seed(). Without that, generation 2 would
		// race ahead and clean up generation 1's ghost on its own, and the
		// assertion would pass whether or not the guard actually stopped anything.
		const gen2 = deferred<ReturnType<typeof rec>[]>();
		const seed = vi
			.fn()
			.mockResolvedValueOnce([rec('a@1')])
			.mockReturnValueOnce(gen2.promise);
		subscribeArrowStream({ connectionId: 'local', seed });
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(2));
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);

		gen2.resolve([rec('b@1')]);
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	// Distinct from the generation trap above: this exercises the `disposed`
	// half of the same guard, with the generation left untouched.
	it('does not write a seed that resolves after dispose', async () => {
		const gen = deferred<ReturnType<typeof rec>[]>();
		const off = subscribeArrowStream({ connectionId: 'local', seed: () => gen.promise });
		off();
		gen.resolve([rec('a@1')]);
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);
	});

	it('ignores a null frame', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0](null);
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);
	});

	// A distinct programming-error scenario from the failed-seed one above: the
	// caller's own onChange throwing must not poison the chain either. Unlike a
	// cache write (upsertArrow/removeArrow), which is documented best-effort
	// and never rejects, an arbitrary caller callback genuinely can throw.
	it('survives onChange throwing during a frame apply and still applies later frames', async () => {
		let calls = 0;
		const onChange = vi.fn(() => {
			calls += 1;
			if (calls === 2) throw new Error('boom');
		});
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange });
		await vi.waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
		subscribers[0]({ event: 'upserted', namespace: 'x@1', name: 'x', description: '', tags: [] });
		await vi.waitFor(() => expect(onChange).toHaveBeenCalledTimes(2));
		subscribers[0]({ event: 'upserted', namespace: 'y@1', name: 'y', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(2));
	});

	// Distinct from "stops applying frames after dispose": that test disposes
	// BEFORE the frame is ever dispatched, so it never enters the chain at all.
	// Here dispose happens WHILE the frame's own write is still in flight, so
	// the write completes (already-enqueued work isn't cancelled) but the
	// onChange notification after it must be swallowed.
	it('does not call onChange for a frame applied after dispose', async () => {
		const cacheMod = await import('./entity-cache');
		const realUpsertArrow = cacheMod.upsertArrow;
		let off: () => void = () => {};
		vi.spyOn(cacheMod, 'upsertArrow').mockImplementationOnce(async (r) => {
			off();
			return realUpsertArrow(r);
		});
		const onChange = vi.fn();
		off = subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange });
		await vi.waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
		subscribers[0]({ event: 'upserted', namespace: 'x@1', name: 'x', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
		expect(onChange).toHaveBeenCalledTimes(1);
	});

	it('stops applying frames after dispose', async () => {
		const done = vi.fn();
		const off = subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		off();
		subscribers[0]({ event: 'upserted', namespace: 'late@1', name: 'late', description: '', tags: [] });
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);
	});
});
