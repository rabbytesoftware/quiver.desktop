import 'fake-indexeddb/auto';
import { IDBFactory } from 'fake-indexeddb';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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

	it('seeds the cache before any frame arrives', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [rec('a@1')], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		expect(await getArrowsFor('local')).toHaveLength(1);
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
		subscribers[0]({ event: 'upserted', namespace: 'new@1', name: 'new', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	// The H21 race: an upsert and a delete for one namespace, each on its own
	// fire-and-forget IDB transaction, can commit out of order — and a late
	// upsert resurrects a tombstoned row. The serial chain is what forbids it.
	//
	// Under a fast fake-indexeddb both operations settle in the same microtask
	// tick regardless of chaining, so a plain dispatch never actually exposes
	// the race (verified by mutation: removing the chain left this test green).
	// The upsert's own IDB write is delayed here so the delete has every
	// opportunity to commit first if arrival order isn't enforced — only the
	// serial chain can still force it to lose that opportunity.
	it('commits an upsert then a delete in arrival order', async () => {
		const cacheMod = await import('./entity-cache');
		const realUpsertArrow = cacheMod.upsertArrow;
		const spy = vi
			.spyOn(cacheMod, 'upsertArrow')
			.mockImplementationOnce((r) => new Promise((resolve) => setTimeout(() => resolve(realUpsertArrow(r)), 20)));
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({ event: 'upserted', namespace: 'x@1', name: 'x', description: '', tags: [] });
		subscribers[0]({ event: 'removed', namespace: 'x@1' });
		// Give the delayed upsert every chance to land before asserting, so this
		// checks the settled final state rather than a lucky early poll.
		await new Promise((r) => setTimeout(r, 40));
		expect(await getArrowsFor('local')).toEqual([]);
		spy.mockRestore();
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
		const spy = vi.spyOn(cacheMod, 'getArrowsFor').mockImplementationOnce(async (id: string) => {
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
		spy.mockRestore();

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
		const spy = vi.spyOn(cacheMod, 'upsertArrow').mockImplementationOnce(async (r) => {
			off();
			return realUpsertArrow(r);
		});
		const onChange = vi.fn();
		off = subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange });
		await vi.waitFor(() => expect(onChange).toHaveBeenCalledTimes(1));
		subscribers[0]({ event: 'upserted', namespace: 'x@1', name: 'x', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
		expect(onChange).toHaveBeenCalledTimes(1);
		spy.mockRestore();
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
