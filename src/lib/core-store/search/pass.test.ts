import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockBackend, type MockRuntime } from '@/lib/mock';
import { useMockStore } from '@/lib/mock/store';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { createSearchController, IDLE_BEFORE_PASS_MS, PASS_DEADLINE_MS, POLL_INTERVAL_MS } from './pass';
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
	vi.useRealTimers();
});

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

	it('never lets two overlapping polls both consume the summary', async () => {
		useMockStore.getState().setLatency(1500);
		const endPassSpy = vi.spyOn(useSearchStore.getState(), 'endPass');

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 15_000);

		expect(endPassSpy).toHaveBeenCalledTimes(1);
		expect(phase()).toBe('settled');
	});

	it('force-settles a pass whose job never reports completion (spec 1.4.1)', async () => {
		const original = mock.backend.fetch.bind(mock.backend);
		const stillRunning = new Response(
			JSON.stringify({
				success: true,
				error: null,
				data: {
					job_id: 'stuck',
					status: 'running',
					query: 'server',
					found: 0,
					verified: 0,
					skipped: 0,
					providers: [],
				},
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
		const fetchSpy = vi.spyOn(mock.backend, 'fetch').mockImplementation((path, init) => {
			if (path.startsWith('/v0/search/discover/')) return Promise.resolve(stillRunning.clone());
			return original(path, init);
		});

		controller.setQuery('server');
		await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + PASS_DEADLINE_MS + POLL_INTERVAL_MS);

		expect(phase()).toBe('settling');
		expect(useSearchStore.getState().passFailed).toBe(true);

		const jobPath = `/v0/search/discover/${useSearchStore.getState().job?.id}`;
		const hub = mock.world.emitter as unknown as { countFor: (path: string) => number };
		expect(hub.countFor(jobPath)).toBe(0);

		const callsSoFar = fetchSpy.mock.calls.filter(([p]) => p === jobPath).length;
		await vi.advanceTimersByTimeAsync(5000);
		expect(fetchSpy.mock.calls.filter(([p]) => p === jobPath).length).toBe(callsSoFar);
	});
});
