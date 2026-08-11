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
		globalThis.indexedDB = new IDBFactory();
		resetDB();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('seeds the cache before any frame arrives', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [rec('a@1')], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		expect(await getArrowsFor('local')).toEqual([rec('a@1')]);
	});

	it('reports a failed seed instead of only logging it', async () => {
		vi.spyOn(console, 'error').mockImplementation(() => undefined);
		const onSeedError = vi.fn();
		const boom = new Error('core unreachable');

		subscribeArrowStream({
			connectionId: 'local',
			seed: () => Promise.reject(boom),
			onChange: vi.fn(),
			onSeedError,
		});

		await vi.waitFor(() => expect(onSeedError).toHaveBeenCalledWith(boom));
	});

	it('stays silent about a seed that failed after disposal', async () => {
		vi.spyOn(console, 'error').mockImplementation(() => undefined);
		const onSeedError = vi.fn();

		const dispose = subscribeArrowStream({
			connectionId: 'local',
			seed: () => Promise.reject(new Error('too late')),
			onSeedError,
		});
		dispose();

		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(onSeedError).not.toHaveBeenCalled();
	});

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

	it('reads icon/banner from a nested media object on a live upsert frame', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({
			event: 'upserted',
			namespace: 'nested@1',
			name: 'Nested',
			description: '',
			tags: [],
			media: { icon: 'nested-icon.png', banner: 'nested-banner.png' },
			version: '1',
		});
		await vi.waitFor(async () =>
			expect(await getArrowsFor('local')).toEqual([
				{
					connectionId: 'local',
					namespace: 'nested@1',
					name: 'Nested',
					description: '',
					tags: [],
					icon: 'nested-icon.png',
					banner: 'nested-banner.png',
					version: '1',
				},
			])
		);
	});

	it('prefers the nested media object over flat icon/banner when a frame carries both', async () => {
		const done = vi.fn();
		subscribeArrowStream({ connectionId: 'local', seed: async () => [], onChange: done });
		await vi.waitFor(() => expect(done).toHaveBeenCalled());
		subscribers[0]({
			event: 'upserted',
			namespace: 'both@1',
			name: 'Both',
			description: '',
			tags: [],
			icon: 'flat-icon.png',
			banner: 'flat-banner.png',
			media: { icon: 'nested-icon.png', banner: 'nested-banner.png' },
			version: '1',
		});
		await vi.waitFor(async () =>
			expect(await getArrowsFor('local')).toEqual([
				{
					connectionId: 'local',
					namespace: 'both@1',
					name: 'Both',
					description: '',
					tags: [],
					icon: 'nested-icon.png',
					banner: 'nested-banner.png',
					version: '1',
				},
			])
		);
	});

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

	it('survives a failed seed and still applies later frames', async () => {
		const seed = vi.fn().mockRejectedValueOnce(new Error('down')).mockResolvedValue([]);
		subscribeArrowStream({ connectionId: 'local', seed });
		await vi.waitFor(() => expect(seed).toHaveBeenCalled());
		subscribers[0]({ event: 'upserted', namespace: 'after@1', name: 'after', description: '', tags: [] });
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	it('a seed superseded by a newer reconnect writes nothing', async () => {
		const gen1 = deferred<ReturnType<typeof rec>[]>();
		const gen2 = deferred<ReturnType<typeof rec>[]>();
		const seed = vi.fn().mockReturnValueOnce(gen1.promise).mockReturnValueOnce(gen2.promise);
		subscribeArrowStream({ connectionId: 'local', seed });
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(1));

		subscribers[0]({ reconnected: true });

		gen1.resolve([rec('a@1')]);
		await vi.waitFor(() => expect(seed).toHaveBeenCalledTimes(2));
		await new Promise((r) => setTimeout(r, 10));
		expect(await getArrowsFor('local')).toEqual([]);

		gen2.resolve([rec('b@1')]);
		await vi.waitFor(async () => expect(await getArrowsFor('local')).toHaveLength(1));
	});

	it('a seed superseded between its GET and its diff read still writes nothing', async () => {
		const cacheMod = await import('./entity-cache');
		const realGetArrowsFor = cacheMod.getArrowsFor;
		vi.spyOn(cacheMod, 'getArrowsFor').mockImplementationOnce(async (id: string) => {
			subscribers[0]({ reconnected: true });
			return realGetArrowsFor(id);
		});
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
