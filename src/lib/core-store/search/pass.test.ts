import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockBackend, type MockRuntime } from '@/lib/mock';
import { useMockStore } from '@/lib/mock/store';
import { installBackend, resetBackend, type SocketLike } from '@/lib/transport/backend';

import { createSearchController, IDLE_BEFORE_PASS_MS, PASS_DEADLINE_MS } from './pass';
import { useSearchStore } from '../store/search';

let mock: MockRuntime;
let controller: ReturnType<typeof createSearchController>;

beforeEach(() => {
	vi.useFakeTimers();
	mock = createMockBackend('normal');
	installBackend(mock.backend);
	useSearchStore.getState().reset();
	controller = createSearchController();
});

afterEach(() => {
	controller.dispose();
	mock.dispose();
	resetBackend();
	useMockStore.getState().setLatency(0);
	useMockStore.getState().resetFaults();
	vi.useRealTimers();
});

/**
 * A promise the test decides when to settle. Latency plus fake timers can put a
 * response on either side of a `dispose()`; this puts it exactly where the test
 * needs it.
 */
function gate<T>(): { promise: Promise<T>; resolve: (value: T) => void; reject: (reason: unknown) => void } {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	// An unhandled rejection here is the controller's job to swallow, not the
	// runner's to report.
	promise.catch(() => {});
	return { promise, resolve, reject };
}

function envelope(data: unknown): Response {
	return new Response(JSON.stringify({ success: true, error: null, data }), {
		status: 200,
		headers: { 'content-type': 'application/json' },
	});
}

const phase = () => useSearchStore.getState().phase;

describe('Lane A', () => {
	it('answers as soon as a query is committed, without waiting for the idle timer', async () => {
		controller.setQuery('minecraft');
		await vi.advanceTimersByTimeAsync(0);
		expect(phase()).toBe('local');
		expect(useSearchStore.getState().local.length).toBeGreaterThan(0);
	});

	it('never calls Lane A with an empty query, because core 400s it', async () => {
		const spy = vi.spyOn(mock.backend, 'fetch');
		controller.setQuery('');
		await vi.advanceTimersByTimeAsync(0);
		expect(spy.mock.calls.filter(([p]) => p.startsWith('/v0/search?'))).toHaveLength(0);
		expect(phase()).toBe('idle');
	});

	it('never calls Lane A with a whitespace-only query either', async () => {
		const spy = vi.spyOn(mock.backend, 'fetch');
		controller.setQuery('   ');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		expect(spy.mock.calls.filter(([p]) => p.startsWith('/v0/search?'))).toHaveLength(0);
		expect(spy.mock.calls.filter(([p]) => p.startsWith('/v0/search/discover'))).toHaveLength(0);
		expect(phase()).toBe('idle');
	});
});

describe('the idle timer', () => {
	it('starts a pass after 600 ms of stillness', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		expect(phase()).toBe('discovering');
	});

	it('re-arms on each committed query, so a pass never fires mid-typing', async () => {
		controller.setQuery('s');
		await vi.advanceTimersByTimeAsync(400);
		controller.setQuery('se');
		await vi.advanceTimersByTimeAsync(400);
		expect(phase()).toBe('local');
		await vi.advanceTimersByTimeAsync(300);
		expect(phase()).toBe('discovering');
	});

	it('fires immediately on submit and clears the timer', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);
		controller.submit();
		await vi.advanceTimersByTimeAsync(10);
		expect(phase()).toBe('discovering');
	});
});

describe('the pass', () => {
	it('streams results into the streamed band', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 600);
		expect(useSearchStore.getState().streamed.length).toBeGreaterThan(0);
	});

	it('settles into one band once the job reports completed', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);
		expect(phase()).toBe('settled');
		expect(useSearchStore.getState().streamed).toEqual([]);
	});

	it('carries every discovered arrow into the re-queried local band, not just the seam (spec 3.5)', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		const discovered = useSearchStore.getState().streamed.map((e) => e.namespace);
		expect(discovered.length).toBeGreaterThan(0);

		await vi.advanceTimersByTimeAsync(10_000);
		expect(phase()).toBe('settled');

		const local = useSearchStore.getState().local;
		for (const namespace of discovered) {
			const entry = local.find((e) => e.namespace === namespace);
			expect(entry).toBeDefined();
			expect(entry).toMatchObject({ installed: false, known: true });
		}
	});

	it('keeps the summary after settling, because the job expires', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);
		const summary = useSearchStore.getState().summary;
		expect(summary?.providers.some((p) => !p.ok)).toBe(true);
		expect(summary?.verified).toBeGreaterThan(0);
	});

	it('cancels the in-flight pass before starting another', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		const firstJob = useSearchStore.getState().job?.id;

		controller.setQuery('server room');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		const secondJob = useSearchStore.getState().job?.id;

		expect(secondJob).toBeDefined();
		expect(secondJob).not.toBe(firstJob);
		// Nothing from the first pass may land in the second pass's band.
		expect(useSearchStore.getState().streamed.every((e) => e.namespace.length > 0)).toBe(true);
	});

	// The generation that guards the socket/poll is bumped by submit, not just
	// by setQuery -- this is what stopPass relies on to keep a cancelled pass's
	// late frames out of the store once submit no longer cancels the local fetch.
	it('cancels an in-flight pass on submit too, not only on a query change', async () => {
		controller.setQuery('server');
		controller.submit();
		await vi.advanceTimersByTimeAsync(400);
		const firstJob = useSearchStore.getState().job?.id;
		expect(firstJob).toBeDefined();

		controller.setQuery('server room');
		controller.submit();
		await vi.advanceTimersByTimeAsync(400);
		const secondJob = useSearchStore.getState().job?.id;

		expect(secondJob).toBeDefined();
		expect(secondJob).not.toBe(firstJob);
		expect(useSearchStore.getState().streamed.every((e) => e.namespace.length > 0)).toBe(true);
	});

	it('stops everything when the query is cleared', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		controller.setQuery('');
		await vi.advanceTimersByTimeAsync(5000);
		expect(phase()).toBe('idle');
		expect(useSearchStore.getState().job).toBeNull();
	});

	it('does not outlive dispose', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		controller.dispose();
		const before = useSearchStore.getState().streamed.length;
		await vi.advanceTimersByTimeAsync(5000);
		expect(useSearchStore.getState().streamed.length).toBe(before);
	});

	it('consumes the summary exactly once, however slow the round trips are', async () => {
		useMockStore.getState().setLatency(1500);
		const endPassSpy = vi.spyOn(useSearchStore.getState(), 'endPass');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 15_000);

		expect(endPassSpy).toHaveBeenCalledTimes(1);
		expect(phase()).toBe('settled');
	});

	it('never polls the job resource while the stream is open', async () => {
		const spy = vi.spyOn(mock.backend, 'fetch');
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 200);
		expect(phase()).toBe('discovering');

		const jobPath = `/v0/search/discover/${useSearchStore.getState().job?.id}`;
		await vi.advanceTimersByTimeAsync(300);
		expect(spy.mock.calls.filter(([p]) => p === jobPath)).toHaveLength(0);
	});

	it('settles on the close frame (1000 completed), then reads the summary once', async () => {
		let opened: SocketLike | null = null;
		const openSocket = mock.backend.openSocket.bind(mock.backend);
		vi.spyOn(mock.backend, 'openSocket').mockImplementation((path) => {
			opened = openSocket(path);
			return opened;
		});
		const spy = vi.spyOn(mock.backend, 'fetch');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		const jobPath = `/v0/search/discover/${useSearchStore.getState().job?.id}`;

		opened!.onclose?.({ code: 1000, reason: 'completed' });
		await vi.advanceTimersByTimeAsync(0);

		expect(phase()).toBe('settled');
		expect(spy.mock.calls.filter(([p]) => p === jobPath)).toHaveLength(1);
	});

	it('still settles when the summary cannot be read after a clean close', async () => {
		let opened: SocketLike | null = null;
		const openSocket = mock.backend.openSocket.bind(mock.backend);
		vi.spyOn(mock.backend, 'openSocket').mockImplementation((path) => {
			opened = openSocket(path);
			return opened;
		});
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path.startsWith('/v0/search/discover/') ? Promise.reject(new Error('gone')) : original(path, init)
		);

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		opened!.onclose?.({ code: 1000, reason: 'completed' });
		await vi.advanceTimersByTimeAsync(0);

		expect(phase()).toBe('settled');
		expect(useSearchStore.getState().summary).toBeNull();
	});

	describe('an abnormal close', () => {
		function captureSockets(): SocketLike[] {
			const sockets: SocketLike[] = [];
			const openSocket = mock.backend.openSocket.bind(mock.backend);
			vi.spyOn(mock.backend, 'openSocket').mockImplementation((path) => {
				const socket = openSocket(path);
				sockets.push(socket);
				return socket;
			});
			return sockets;
		}

		it('reconnects once on 1013 (slow consumer) and settles when the replay completes', async () => {
			const sockets = captureSockets();
			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
			expect(sockets).toHaveLength(1);

			sockets[0].onclose?.({ code: 1013, reason: 'slow consumer' });
			expect(sockets).toHaveLength(2);
			expect(phase()).toBe('discovering');

			sockets[1].onclose?.({ code: 1000, reason: 'completed' });
			await vi.advanceTimersByTimeAsync(0);
			expect(phase()).toBe('settled');
		});

		it('does not duplicate a namespace the replay delivers again', async () => {
			const sockets = captureSockets();
			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);

			const frame = JSON.stringify({ namespace: 'github.com/a/b', name: 'b', description: '', tags: [] });
			sockets[0].onmessage?.({ data: frame });
			sockets[0].onclose?.({ code: 1013, reason: 'slow consumer' });
			sockets[1].onmessage?.({ data: frame });

			const streamed = useSearchStore.getState().streamed.filter((e) => e.namespace === 'github.com/a/b');
			expect(streamed).toHaveLength(1);
		});

		it('fails the pass when the second stream breaks too, keeping what it has', async () => {
			const sockets = captureSockets();
			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);

			sockets[0].onclose?.();
			sockets[1].onclose?.({ code: 1006, reason: '' });

			expect(sockets).toHaveLength(2);
			expect(phase()).toBe('settling');
			expect(useSearchStore.getState().passFailed).toBe(true);
		});

		it('gives up after the safety deadline if a reconnected stream never closes', async () => {
			(mock.world.emitter as { close?: unknown }).close = () => {};
			const sockets = captureSockets();
			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
			sockets[0].onclose?.({ code: 1013, reason: 'slow consumer' });

			await vi.advanceTimersByTimeAsync(PASS_DEADLINE_MS + 10);

			expect(phase()).toBe('settling');
			expect(useSearchStore.getState().passFailed).toBe(true);
		});

		it('does not fail a pass that completed inside the safety window', async () => {
			const sockets = captureSockets();
			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
			sockets[0].onclose?.({ code: 1013, reason: 'slow consumer' });
			sockets[1].onclose?.({ code: 1000, reason: 'completed' });

			await vi.advanceTimersByTimeAsync(PASS_DEADLINE_MS + 10);

			expect(phase()).toBe('settled');
			expect(useSearchStore.getState().passFailed).toBe(false);
		});
	});

	describe('the os filter', () => {
		it('sends the same platform on Lane A, the stream and the re-query', async () => {
			const platform = await mock.backend.getPlatform();
			const fetchSpy = vi.spyOn(mock.backend, 'fetch');
			const socketSpy = vi.spyOn(mock.backend, 'openSocket');

			controller.setQuery('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);

			const encoded = encodeURIComponent(platform);
			const lanePaths = fetchSpy.mock.calls.map(([p]) => p).filter((p) => p.startsWith('/v0/search?'));
			expect(lanePaths.length).toBeGreaterThanOrEqual(2);
			for (const path of lanePaths) expect(path).toContain(`&os=${encoded}`);
			expect(socketSpy.mock.calls[0][0]).toMatch(/^\/v0\/search\/discover\/[^?]+\?os=/);
			expect(socketSpy.mock.calls[0][0]).toContain(`?os=${encoded}`);
		});
	});
});

// Reopening a search is not asking for it again. The arrows a pass found are in
// the vault, so Lane A alone can rebuild the screen -- going back to the git
// hosts every time the field is focused is a 10s round trip for the same rows.
describe('restoring a query', () => {
	it('rebuilds the screen from Lane A without going back to the network', async () => {
		const spy = vi.spyOn(mock.backend, 'fetch');

		controller.setQuery('minecraft', { discover: false });
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);

		expect(useSearchStore.getState().local.length).toBeGreaterThan(0);
		expect(spy.mock.calls.filter(([p]) => p.startsWith('/v0/search/discover'))).toHaveLength(0);
		expect(phase()).toBe('local');
	});

	it('still fires a pass for the next query the user actually types', async () => {
		controller.setQuery('minecraft', { discover: false });
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		expect(phase()).toBe('local');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		expect(phase()).toBe('discovering');
	});

	it('still fires a pass on Enter, which is an explicit ask', async () => {
		controller.setQuery('minecraft', { discover: false });
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
		expect(phase()).toBe('local');

		controller.submit();
		await vi.advanceTimersByTimeAsync(50);
		expect(phase()).toBe('discovering');
	});
});

// The store is a module singleton; a controller lives and dies with one mount
// of the screen. Nothing may survive that gap -- a query with results attached
// to it is the screen's state, not the app's. `useSearch` disposes in the same
// effect cleanup that precedes the next setup, so the handover is always
// dispose-then-create and this is the only seam to hold.
describe('the gap between two screens', () => {
	it('leaves no results behind for the next screen to inherit', async () => {
		controller.setQuery('minecraft');
		await vi.advanceTimersByTimeAsync(0);
		expect(useSearchStore.getState().local.length).toBeGreaterThan(0);

		controller.dispose();

		expect(useSearchStore.getState().local).toEqual([]);
		expect(useSearchStore.getState().streamed).toEqual([]);
		expect(phase()).toBe('idle');
	});

	it('keeps an Enter that is already in flight, which belongs to the field, not the screen', async () => {
		useSearchStore.getState().requestSubmit('minecraft');
		controller.dispose();
		expect(useSearchStore.getState().submitQuery).toBe('minecraft');
	});
});

/**
 * Every `await` in the controller is a window in which the user can type again,
 * hit Enter, or leave the screen. Both lanes are generation-guarded on the far
 * side of that window; without it the screen shows the answer to a question
 * nobody asked any more. Each of these holds a response open across the event
 * that invalidates it, then lets it land.
 */
describe('answers that arrive too late', () => {
	function holdLaneA(): ReturnType<typeof gate<Response>> {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path.startsWith('/v0/search?') ? held.promise : original(path, init)
		);
		return held;
	}

	it('drops a Lane A answer that lands after dispose', async () => {
		const held = holdLaneA();
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);

		controller.dispose();
		held.resolve(envelope([]));
		await vi.advanceTimersByTimeAsync(0);

		expect(useSearchStore.getState().local).toEqual([]);
		expect(phase()).toBe('idle');
	});

	it('drops a Lane A failure that lands after dispose', async () => {
		const held = holdLaneA();
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);

		controller.dispose();
		held.reject(new Error('too late'));
		await vi.advanceTimersByTimeAsync(0);

		// setLocalError would have moved the screen to a failure it can no longer
		// act on -- the controller is gone.
		expect(phase()).toBe('idle');
		expect(useSearchStore.getState().localError).toBe(false);
	});

	it('drops a Lane A failure for a query the user has already replaced', async () => {
		const held = holdLaneA();
		controller.setQuery('serv');
		await vi.advanceTimersByTimeAsync(0);

		// Second query goes to the real mock and answers normally.
		vi.mocked(mock.backend.fetch).mockRestore();
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);
		expect(phase()).toBe('local');

		held.reject(new Error('stale'));
		await vi.advanceTimersByTimeAsync(0);

		// The stale failure must not overwrite the answer that is on screen.
		expect(phase()).toBe('local');
		expect(useSearchStore.getState().local.length).toBeGreaterThan(0);
	});

	it('drops a settle re-query that lands after dispose', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		let passStarted = false;
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) => {
			// Only intercept the re-query, which is the Lane A path called again
			// after the job reports completed.
			if (passStarted && path.startsWith('/v0/search?')) return held.promise;
			if (path.startsWith('/v0/search/discover')) passStarted = true;
			return original(path, init);
		});

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);
		expect(phase()).toBe('settling');

		// `dispose` clears the store, so the assertion is that it STAYS cleared --
		// `settle` would put the phase at 'settled' and refill the local band.
		controller.dispose();
		held.resolve(envelope([]));
		await vi.advanceTimersByTimeAsync(0);

		expect(phase()).toBe('idle');
		expect(useSearchStore.getState().local).toEqual([]);
	});

	it('drops a failed settle re-query that lands after dispose', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		let passStarted = false;
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) => {
			if (passStarted && path.startsWith('/v0/search?')) return held.promise;
			if (path.startsWith('/v0/search/discover')) passStarted = true;
			return original(path, init);
		});

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);

		controller.dispose();
		held.reject(new Error('too late'));
		await vi.advanceTimersByTimeAsync(0);

		expect(useSearchStore.getState().passFailed).toBe(false);
		expect(phase()).toBe('idle');
	});

	it('drops a settle re-query for a pass that has already been replaced', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		let passStarted = false;
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) => {
			if (passStarted && path.startsWith('/v0/search?')) return held.promise;
			if (path.startsWith('/v0/search/discover')) passStarted = true;
			return original(path, init);
		});

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);
		expect(phase()).toBe('settling');

		// Enter starts a new pass, which bumps the generation the held re-query
		// was issued under.
		controller.submit();
		held.reject(new Error('stale'));
		await vi.advanceTimersByTimeAsync(0);

		// The new pass owns the screen now; the old re-query's failure must not
		// stamp `passFailed` onto it.
		expect(useSearchStore.getState().passFailed).toBe(false);
		expect(phase()).toBe('discovering');
	});

	it('does not begin a pass whose POST never came back with a ticket', async () => {
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path === '/v0/search/discover' ? Promise.reject(new Error('refused')) : original(path, init)
		);

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 2000);

		// No job, and no socket opened for a job id that does not exist.
		expect(useSearchStore.getState().job).toBeNull();
		expect(phase()).toBe('local');
	});

	it('does not begin a pass whose ticket lands after dispose', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path === '/v0/search/discover' ? held.promise : original(path, init)
		);
		const openSocket = vi.spyOn(mock.backend, 'openSocket');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);

		controller.dispose();
		held.resolve(envelope({ job_id: 'late', query: 'server', expires_at: new Date().toISOString() }));
		await vi.advanceTimersByTimeAsync(0);

		expect(useSearchStore.getState().job).toBeNull();
		expect(openSocket).not.toHaveBeenCalled();
	});

	it('drops a streamed frame delivered after the pass was replaced', async () => {
		let opened: SocketLike | null = null;
		const openSocket = mock.backend.openSocket.bind(mock.backend);
		vi.spyOn(mock.backend, 'openSocket').mockImplementation((path) => {
			opened = openSocket(path);
			return opened;
		});

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		expect(opened).not.toBeNull();

		// Enter replaces the pass; the old socket's handler is still reachable.
		controller.submit();
		const before = useSearchStore.getState().streamed.length;

		opened!.onmessage?.({
			data: JSON.stringify({ namespace: 'github.com/late/frame', name: 'frame', refs: [] }),
		} as MessageEvent);

		expect(useSearchStore.getState().streamed.length).toBe(before);
	});

	async function startPassWithSocket(): Promise<SocketLike> {
		let opened: SocketLike | null = null;
		const openSocket = mock.backend.openSocket.bind(mock.backend);
		vi.spyOn(mock.backend, 'openSocket').mockImplementation((path) => {
			opened = openSocket(path);
			return opened;
		});
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10);
		return opened!;
	}

	it('ignores a close frame from a socket the controller already let go of', async () => {
		const socket = await startPassWithSocket();
		const spy = vi.spyOn(mock.backend, 'fetch');
		controller.dispose();

		socket.onclose?.({ code: 1000, reason: 'completed' });
		await vi.advanceTimersByTimeAsync(0);

		expect(spy.mock.calls.filter(([p]) => p.startsWith('/v0/search/discover/'))).toHaveLength(0);
		expect(phase()).toBe('idle');
	});

	it('drops a summary that lands after dispose', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path.startsWith('/v0/search/discover/') ? held.promise : original(path, init)
		);
		const socket = await startPassWithSocket();

		socket.onclose?.({ code: 1000, reason: 'completed' });
		controller.dispose();
		held.resolve(
			envelope({
				job_id: 'x',
				status: 'completed',
				query: 'server',
				found: 0,
				verified: 0,
				skipped: 0,
				providers: [],
			})
		);
		await vi.advanceTimersByTimeAsync(0);

		expect(phase()).toBe('idle');
		expect(useSearchStore.getState().summary).toBeNull();
	});

	it('drops a failed summary read that lands after dispose', async () => {
		const held = gate<Response>();
		const original = mock.backend.fetch.bind(mock.backend);
		vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) =>
			path.startsWith('/v0/search/discover/') ? held.promise : original(path, init)
		);
		const socket = await startPassWithSocket();

		socket.onclose?.({ code: 1000, reason: 'completed' });
		controller.dispose();
		held.reject(new Error('gone'));
		await vi.advanceTimersByTimeAsync(0);

		expect(useSearchStore.getState().passFailed).toBe(false);
		expect(phase()).toBe('idle');
	});

	it('never arms a pass that would fire after dispose', async () => {
		const openSocket = vi.spyOn(mock.backend, 'openSocket');
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS - 100);

		controller.dispose();
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 1000);

		expect(openSocket).not.toHaveBeenCalled();
	});

	it('ignores a query set after dispose', async () => {
		controller.dispose();
		const spy = vi.spyOn(mock.backend, 'fetch');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 1000);

		expect(spy).not.toHaveBeenCalled();
		expect(phase()).toBe('idle');
	});

	it('ignores a query that has not actually changed', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);
		const spy = vi.spyOn(mock.backend, 'fetch');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);

		// Re-committing the same query would cancel the pass it just armed.
		expect(spy).not.toHaveBeenCalled();
	});

	it('ignores submit after dispose', async () => {
		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(0);
		controller.dispose();
		const spy = vi.spyOn(mock.backend, 'fetch');

		controller.submit();
		await vi.advanceTimersByTimeAsync(1000);

		expect(spy).not.toHaveBeenCalled();
	});
});
