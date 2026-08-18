import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockBackend, type MockRuntime } from '../index';
import { useMockStore } from '../store';

const NS = 'github.com/rabbyte';
const MINECRAFT = `${NS}/minecraft@v1.21.4`;
const POSTGRES = `${NS}/postgres@v17.2`;
const GAME_SERVERS = `${NS}/game-servers`;

let mock: MockRuntime;

function enc(ns: string): string {
	return encodeURIComponent(ns);
}

async function call(method: string, path: string, body?: unknown) {
	const res = await mock.backend.fetch(path, {
		method,
		...(body ? { body: JSON.stringify(body) } : {}),
	});
	const text = await res.text();
	return { status: res.status, body: text ? (JSON.parse(text) as Record<string, unknown>) : null };
}

beforeEach(() => {
	useMockStore.setState({ latency: 0, errorRate: 0, unreachable: false });
	useMockStore.getState().resetFaults();
	mock = createMockBackend('normal');
});

afterEach(() => {
	mock.dispose();
	vi.useRealTimers();
});

describe('collections', () => {
	it('lists them with a member count rather than the members', async () => {
		const { body } = await call('GET', '/v0/collection');
		const list = body!.data as Array<{ namespace: string; arrow_count: number }>;
		expect(list).toHaveLength(2);
		expect(list.find((c) => c.namespace === GAME_SERVERS)?.arrow_count).toBe(4);
	});

	it('follows and unfollows', async () => {
		expect(mock.world.collections.get(`${NS}/homelab`)!.followed).toBe(false);
		await call('POST', `/v0/collection/${enc(`${NS}/homelab`)}/follow`);
		expect(mock.world.collections.get(`${NS}/homelab`)!.followed).toBe(true);
		await call('DELETE', `/v0/collection/${enc(`${NS}/homelab`)}/follow`);
		expect(mock.world.collections.get(`${NS}/homelab`)!.followed).toBe(false);
	});

	it('404s every route for a collection that does not exist', async () => {
		expect((await call('GET', `/v0/collection/${enc('nope/nope')}`)).status).toBe(404);
		expect((await call('POST', `/v0/collection/${enc('nope/nope')}/follow`)).status).toBe(404);
		expect((await call('DELETE', `/v0/collection/${enc('nope/nope')}/follow`)).status).toBe(404);
	});
});

describe('library membership', () => {
	it('404s an arrow the world has never heard of', async () => {
		expect((await call('GET', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
		expect((await call('POST', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
		expect((await call('DELETE', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
	});

	it('500s a POST for an arrow already in the library, the way core does', async () => {
		const { status } = await call('POST', `/v0/arrow/${enc(MINECRAFT)}`);
		expect(status).toBe(500);
	});
});

describe('runtime refusals', () => {
	beforeEach(() => vi.useFakeTimers());

	it('404s an unknown arrow', async () => {
		expect((await call('POST', `/v0/runtime/${enc('nope/nope@v1')}/install`)).status).toBe(404);
	});

	it('404s a verb it does not implement', async () => {
		expect((await call('POST', `/v0/runtime/${enc(POSTGRES)}/frobnicate`)).status).toBe(404);
	});

	it('refuses to stop something that is not running', async () => {
		const { status, body } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/stop`);
		expect(status).toBe(409);
		expect(body!.error).toMatch(/not running/);
	});

	it('requires _execute to name a method', async () => {
		const { status, body } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/_execute`, {});
		expect(status).toBe(400);
		expect(body!.error).toMatch(/method name/);
	});

	it('404s a method the arrow does not declare', async () => {
		const { status } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/_execute`, { method: 'teleport' });
		expect(status).toBe(404);
	});

	it('refuses any action on an arrow already mid-transition', async () => {
		const { status, body } = await call('POST', `/v0/runtime/${enc(`${NS}/redis@v7.4.1`)}/install`);
		expect(status).toBe(409);
		expect(body!.error).toMatch(/installing/);
	});
});

describe('runtime verbs that do run', () => {
	beforeEach(() => vi.useFakeTimers());

	it('uninstall walks its own steps and lands back on absent', async () => {
		const arrow = mock.world.arrows.get(POSTGRES)!;
		const { status } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/uninstall`);
		expect(status).toBe(202);
		expect(arrow.state).toBe('uninstalling');

		await vi.advanceTimersByTimeAsync(700 * 5);
		expect(arrow.state).toBe('absent');
	});

	it('stop takes a running arrow back to ready', async () => {
		const arrow = mock.world.arrows.get(MINECRAFT)!;
		await call('POST', `/v0/runtime/${enc(MINECRAFT)}/stop`);
		expect(arrow.state).toBe('stopping');

		await vi.advanceTimersByTimeAsync(700 * 4);
		expect(arrow.state).toBe('ready');
	});

	it('a non-start method leaves the state where it found it', async () => {
		const arrow = mock.world.arrows.get(MINECRAFT)!;
		expect(arrow.state).toBe('running');

		await call('POST', `/v0/runtime/${enc(MINECRAFT)}/_execute`, { method: 'backup' });
		await vi.advanceTimersByTimeAsync(700 * 5);

		expect(arrow.state).toBe('running');
		expect(arrow.last_return?.method).toBe('backup');
	});

	it('carries the submitted variables into the run and its return', async () => {
		const arrow = mock.world.arrows.get(POSTGRES)!;
		arrow.state = 'absent';
		await call('POST', `/v0/runtime/${enc(POSTGRES)}/install`, {
			variables: { POSTGRES_USER: 'char2cs' },
		});
		expect(arrow.active_run?.variables).toEqual({ POSTGRES_USER: 'char2cs' });

		await vi.advanceTimersByTimeAsync(700 * 7);
		expect(arrow.last_return?.variables).toEqual({ POSTGRES_USER: 'char2cs' });
	});
});

describe('search', () => {
	it('rejects an empty query rather than answering with the whole shelf', async () => {
		expect((await call('GET', '/v0/search')).status).toBe(400);
	});

	it('matches on tag as well as name and description', async () => {
		const byTag = (await call('GET', '/v0/search?q=database')).body!.data as Array<{ name: string }>;
		expect(byTag.map((r) => r.name)).toContain('PostgreSQL');
	});

	it('omits state and ports, matching core SearchResultDTO', async () => {
		const hits = (await call('GET', '/v0/search?q=minecraft')).body!.data as Array<Record<string, unknown>>;
		expect(hits[0]).not.toHaveProperty('state');
		expect(hits[0]).not.toHaveProperty('netbridge');
	});

	it('404s a discovery job id it never issued', async () => {
		expect((await call('GET', '/v0/search/discover/job-999')).status).toBe(404);
	});
});

describe('the router itself', () => {
	it('turns a throwing handler into a 500 that names the route', async () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		mock.world.arrows.set('boom@v1', null as never);

		const { status, body } = await call('GET', '/v0/arrow');
		expect(status).toBe(500);
		expect(body!.error).toMatch(/mock handler error/);
	});

	it('keeps a malformed body from taking the request down', async () => {
		const res = await mock.backend.fetch(`/v0/runtime/${enc(POSTGRES)}/_execute`, {
			method: 'POST',
			body: 'not json at all',
		});
		expect(res.status).toBe(400);
	});

	it('does not match a route of a different depth', async () => {
		expect((await call('GET', '/v0/arrow/a/b/c')).status).toBe(404);
	});
});

describe('chaos, at the edges', () => {
	it('is inert with every knob at zero', async () => {
		expect((await call('GET', '/v0/arrow')).status).toBe(200);
	});

	it('applies the error rate to any route, unlike a per-route fault', async () => {
		useMockStore.getState().setErrorRate(100);
		expect((await call('GET', '/v0/collection')).status).toBe(500);
		expect((await call('GET', '/v0/search')).status).toBe(500);
	});

	it('beats the error rate, and marks itself as the proxy', async () => {
		useMockStore.getState().setErrorRate(100);
		useMockStore.getState().setUnreachable(true);
		const res = await mock.backend.fetch('/v0/arrow');
		expect(res.status).toBe(502);
		expect(res.headers.get('x-quiver-proxy')).toBe('error');
	});

	it('takes even the health probe down when unreachable', async () => {
		useMockStore.getState().setUnreachable(true);
		expect((await mock.backend.fetch('/v0/health')).status).toBe(502);
	});
});
