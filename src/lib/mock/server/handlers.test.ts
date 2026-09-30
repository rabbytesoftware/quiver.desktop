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
		expect(list).toHaveLength(3);
		expect(list.find((c) => c.namespace === GAME_SERVERS)?.arrow_count).toBe(4);
	});

	it('filters to only followed collections when asked', async () => {
		const { body } = await call('GET', '/v0/collection?followed=true');
		const list = body!.data as Array<{ namespace: string; followed: boolean }>;
		expect(list.length).toBeGreaterThan(0);
		expect(list.every((c) => c.followed)).toBe(true);
		expect(list.find((c) => c.namespace === `${NS}/homelab`)).toBeUndefined();
	});

	it('filters to only unfollowed collections when asked', async () => {
		const { body } = await call('GET', '/v0/collection?followed=false');
		const list = body!.data as Array<{ namespace: string; followed: boolean }>;
		expect(list.every((c) => !c.followed)).toBe(true);
		expect(list.find((c) => c.namespace === `${NS}/homelab`)).toBeDefined();
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

	it('carries media, url, and per-member names on a collection that has them', async () => {
		const { body } = await call('GET', `/v0/collection/${enc(GAME_SERVERS)}`);
		const detail = body!.data as {
			url?: string;
			media?: { banner?: string };
			arrows: Array<{ namespace: string; name?: string }>;
		};
		expect(detail.media?.banner).toBeTruthy();
		expect(detail.url).toBe(`https://${GAME_SERVERS}`);
		expect(detail.arrows.find((a) => a.namespace === MINECRAFT)?.name).toBe('Minecraft Server');
	});

	it('omits url entirely for a collection that has none', async () => {
		const { body } = await call('GET', `/v0/collection/${enc(`${NS}/homelab`)}`);
		expect(JSON.stringify(body!.data)).not.toContain('"url"');
	});

	it('omits media entirely for a collection that has none', async () => {
		const { body } = await call('GET', `/v0/collection/${enc(`${NS}/homelab`)}`);
		expect(JSON.stringify(body!.data)).not.toContain('"media"');
	});
});

describe('library membership', () => {
	it('404s an arrow the world has never heard of', async () => {
		expect((await call('GET', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
		expect((await call('POST', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
		expect((await call('DELETE', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
	});

	it('answers a re-registration of an identity already in the library as the no-op core makes it', async () => {
		const { status } = await call('POST', `/v0/arrow/${enc(MINECRAFT)}`);
		expect(status).toBe(201);
	});

	it('resolves GET /v0/arrow/:ns by a bare namespace too, reporting the identity it reached', async () => {
		const { status, body } = await call('GET', `/v0/arrow/${enc('github.com/rabbyte/minecraft')}`);
		expect(status).toBe(200);
		expect((body!.data as { namespace: string }).namespace).toBe(MINECRAFT);
	});

	it('registers library membership by a bare namespace too -- Search links to a Discovered arrow with no installed ref to offer', async () => {
		const bare = `${NS}/mariadb`;
		const { status } = await call('POST', `/v0/arrow/${enc(bare)}`);
		expect(status).toBe(201);
		expect(mock.world.arrows.get(`${bare}@v11.6.2`)!.user_installed).toBe(true);
	});

	it('files a registration under the selector in its path, as a new row beside any other', async () => {
		const { status } = await call('POST', `/v0/arrow/${enc(`${NS}/minecraft@nightly`)}`);
		expect(status).toBe(201);
		const row = mock.world.arrows.get(`${NS}/minecraft@nightly`)!;
		expect(row.user_installed).toBe(true);
		expect(row.state).toBe('absent');
		expect(mock.world.arrows.get(MINECRAFT)).toBeDefined();

		const { body } = await call('GET', `/v0/arrow/${enc(`${NS}/minecraft@nightly`)}`);
		expect(body!.data).toMatchObject({
			namespace: `${NS}/minecraft@nightly`,
			selector_kind: 'channel',
			resolved_ref: 'nightly-latest',
			outdated: false,
		});
	});

	it.each([
		['v1.*', 'constraint'],
		['abcdef1', 'commit'],
		['v1.20.6', 'pin'],
	])('classifies a registered selector %s as a %s, the way core does', async (selector, kind) => {
		await call('POST', `/v0/arrow/${enc(`${NS}/minecraft@${selector}`)}`);
		const { body } = await call('GET', `/v0/arrow/${enc(`${NS}/minecraft@${selector}`)}`);
		expect(body!.data).toMatchObject({ selector_kind: kind });
	});

	it('names the identity it filed a registration under, as core’s mutation envelope does', async () => {
		const { body } = await call('POST', `/v0/arrow/${enc(`${NS}/minecraft@nightly`)}`);
		expect(body).toMatchObject({ success: true, namespace: `${NS}/minecraft@nightly` });
	});

	it('adopts a declared installed ref under the identity, in place, keeping it in the library', async () => {
		const { status, body } = await call('POST', `/v0/arrow/${enc(`${NS}/terraria@v1.4.4.9`)}/adopt`, {
			resolved_ref: 'v1.4.5.0',
		});
		expect(status).toBe(201);
		expect(body).toMatchObject({ namespace: `${NS}/terraria@v1.4.4.9` });
		const arrow = mock.world.arrows.get(`${NS}/terraria@v1.4.4.9`)!;
		expect(arrow.resolved_ref).toBe('v1.4.5.0');
		expect(arrow.available).toBeUndefined();
		expect(arrow.user_installed).toBe(true);
	});

	it('adopts into a new row for an identity the world has no row for yet', async () => {
		await call('POST', `/v0/arrow/${enc(`${NS}/minecraft@stable`)}/adopt`, { resolved_ref: 'v1.21.1' });
		const row = mock.world.arrows.get(`${NS}/minecraft@stable`)!;
		expect(row.resolved_ref).toBe('v1.21.1');
		expect(row.selector_kind).toBe('channel');
	});

	it('400s an adopt with no resolved_ref, and 404s one for a repository it has never heard of', async () => {
		expect((await call('POST', `/v0/arrow/${enc(MINECRAFT)}/adopt`, {})).status).toBe(400);
		expect((await call('POST', `/v0/arrow/${enc('nope/nope@stable')}/adopt`, { resolved_ref: 'v1' })).status).toBe(
			404
		);
	});

	it('404s a registration of a repository the world has never heard of, whatever the selector', async () => {
		expect((await call('POST', `/v0/arrow/${enc('nope/nope@stable')}`)).status).toBe(404);
	});
});

describe('the detail and manifest wire shape', () => {
	it('reports a pinned row with its resolved ref and nothing available', async () => {
		const { body } = await call('GET', `/v0/arrow/${enc(MINECRAFT)}`);
		const detail = body!.data as Record<string, unknown>;
		expect(detail).toMatchObject({
			namespace: MINECRAFT,
			selector_kind: 'pin',
			resolved_ref: 'v1.21.4',
			outdated: false,
		});
		expect(detail).not.toHaveProperty('available');
		for (const removed of ['installed_ref', 'installed_constraint', 'channel', 'recommended_ref', 'version']) {
			expect(detail).not.toHaveProperty(removed);
		}
	});

	it('reports what an outdated row has ahead of it', async () => {
		const { body } = await call('GET', `/v0/arrow/${enc(`${NS}/terraria@v1.4.4.9`)}`);
		expect(body!.data).toMatchObject({ outdated: true, available: { ref: 'v1.4.5.0' } });
	});

	it('nests the author’s metadata under manifest.metadata', async () => {
		const { body } = await call('GET', `/v0/arrow/${enc(MINECRAFT)}/manifest`);
		const { manifest } = body!.data as { manifest: Record<string, unknown> };
		expect(Object.keys(manifest).sort()).toEqual(['metadata', 'netbridge', 'readme', 'targets', 'variables']);
		expect(manifest.metadata).toMatchObject({
			name: 'Minecraft Server',
			url: 'https://github.com/rabbyte/minecraft',
		});
	});

	it('lists each row under its identity selector with the ref it resolved to', async () => {
		const { body } = await call('GET', '/v0/arrow');
		const minecraft = (body!.data as Array<{ namespace: string; versions: unknown[] }>).find(
			(a) => a.namespace === `${NS}/minecraft`
		)!;
		expect(minecraft.versions[0]).toMatchObject({ ref: 'v1.21.4', resolved_ref: 'v1.21.4' });
		expect(minecraft.versions[0]).not.toHaveProperty('version');
		expect(minecraft.versions[0]).not.toHaveProperty('constraint');
	});

	it('serves the readme at a full identity', async () => {
		expect((await call('GET', `/v0/arrow/${enc(MINECRAFT)}/readme`)).status).toBe(200);
	});
});

describe('channels', () => {
	it('lists the ordered and pointer channels a fixture publishes', async () => {
		const { status, body } = await call('GET', `/v0/arrow/${enc(MINECRAFT)}/channels`);
		expect(status).toBe(200);
		const { channels } = body!.data as {
			channels: Array<{ name: string; kind: string; latest: string; count?: number; members?: string[] }>;
		};
		expect(channels).toEqual([
			{
				name: 'stable',
				kind: 'ordered',
				latest: 'v1.21.4',
				count: 3,
				members: ['v1.21.4', 'v1.21.1', 'v1.20.6'],
			},
			{ name: 'nightly', kind: 'pointer', latest: 'nightly-latest' },
		]);
	});

	it('reports an empty list for an arrow that publishes none', async () => {
		const { status, body } = await call('GET', `/v0/arrow/${enc(POSTGRES)}/channels`);
		expect(status).toBe(200);
		expect(body!.data).toEqual({ channels: [] });
	});

	it('404s for an arrow the world has never heard of', async () => {
		expect((await call('GET', `/v0/arrow/${enc('nope/nope@v1')}/channels`)).status).toBe(404);
	});

	it('re-checks with a PATCH that takes no body, reporting what is available', async () => {
		const { status, body } = await call('PATCH', `/v0/arrow/${enc(`${NS}/terraria@v1.4.4.9`)}`);
		expect(status).toBe(200);
		expect(body!.data).toMatchObject({ added_deps: [], available: { ref: 'v1.4.5.0' } });
		const detail = await call('GET', `/v0/arrow/${enc(`${NS}/terraria@v1.4.4.9`)}`);
		expect(detail.body!.data).toMatchObject({ resolved_ref: 'v1.4.4.9', outdated: true });
	});

	it('advances a row nothing is installed from in place, keeping its identity', async () => {
		const arrow = mock.world.arrows.get(`${NS}/terraria@v1.4.4.9`)!;
		arrow.state = 'absent';
		const { body } = await call('PATCH', `/v0/arrow/${enc(`${NS}/terraria@v1.4.4.9`)}`);
		expect(body!.data).not.toHaveProperty('available');
		expect(arrow.resolved_ref).toBe('v1.4.5.0');
		expect(arrow.available).toBeUndefined();
		expect(mock.world.arrows.get(`${NS}/terraria@v1.4.4.9`)).toBe(arrow);
	});

	it('reports nothing available for a current row', async () => {
		const { body } = await call('PATCH', `/v0/arrow/${enc(POSTGRES)}`);
		expect(body!.data).not.toHaveProperty('available');
	});

	it('404s a PATCH for an arrow the world has never heard of', async () => {
		expect((await call('PATCH', `/v0/arrow/${enc('nope/nope@v1')}`)).status).toBe(404);
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

	it('refuses execute outside ready, unconditionally -- no manifest override exists for it', async () => {
		const { status, body } = await call('POST', `/v0/runtime/${enc(MINECRAFT)}/execute`, {});
		expect(status).toBe(409);
		expect(body!.error).toMatch(/ready/);
	});

	it('404s a custom method the arrow does not declare, invoked by its own name', async () => {
		const { status } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/teleport`, {});
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

	// Mirrors quiver.core's real BeginStop.Validate: a detached arrow's only
	// recovery path is this same plain stop call, not a distinct "force" verb.
	it('stop also recovers a detached arrow back to ready', async () => {
		const arrow = mock.world.arrows.get(`${NS}/syncthing@v1.28.1`)!;
		expect(arrow.state).toBe('detached');

		const { status } = await call('POST', `/v0/runtime/${enc(`${NS}/syncthing@v1.28.1`)}/stop`);
		expect(status).toBe(202);
		expect(arrow.state).toBe('stopping');

		await vi.advanceTimersByTimeAsync(700 * 4);
		expect(arrow.state).toBe('ready');
	});

	it('a custom method, invoked by its own name, leaves the state where it found it', async () => {
		const arrow = mock.world.arrows.get(MINECRAFT)!;
		expect(arrow.state).toBe('running');

		await call('POST', `/v0/runtime/${enc(MINECRAFT)}/backup`, {});
		await vi.advanceTimersByTimeAsync(700 * 5);

		expect(arrow.state).toBe('running');
		expect(arrow.last_return?.method).toBe('backup');
	});

	it('execute walks the shared go-action steps and lands on running, with a pid', async () => {
		const arrow = mock.world.arrows.get(POSTGRES)!;
		arrow.state = 'ready';
		const { status } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/execute`, {});
		expect(status).toBe(202);
		expect(arrow.state).toBe('ready');

		await vi.advanceTimersByTimeAsync(700 * 5);
		expect(arrow.state).toBe('running');
		expect(arrow.active_run?.pid).toBeGreaterThan(0);
	});

	it('update walks its own steps, lands back on ready, and advances the same row to what was available', async () => {
		const arrow = mock.world.arrows.get(`${NS}/terraria@v1.4.4.9`)!;
		expect(arrow.state).toBe('outdated');
		const { status } = await call('POST', `/v0/runtime/${enc(`${NS}/terraria@v1.4.4.9`)}/update`, {});
		expect(status).toBe(202);
		expect(arrow.state).toBe('updating');

		await vi.advanceTimersByTimeAsync(700 * 6);
		expect(arrow.state).toBe('ready');
		expect(arrow.resolved_ref).toBe('v1.4.5.0');
		expect(arrow.available).toBeUndefined();
		expect(mock.world.arrows.get(`${NS}/terraria@v1.4.4.9`)).toBe(arrow);
	});

	it('answers an update with nothing newer as a 200 no-op that starts nothing', async () => {
		const arrow = mock.world.arrows.get(POSTGRES)!;
		arrow.state = 'ready';
		const { status } = await call('POST', `/v0/runtime/${enc(POSTGRES)}/update`, {});
		expect(status).toBe(200);
		expect(arrow.state).toBe('ready');
		expect(arrow.active_run).toBeNull();
	});

	it('refuses update outside ready/outdated', async () => {
		// MINECRAFT is 'running' -- past the broad STARTABLE gate, so this
		// exercises update's own narrower check specifically.
		const { status, body } = await call('POST', `/v0/runtime/${enc(MINECRAFT)}/update`, {});
		expect(status).toBe(422);
		expect(body!.error).toMatch(/ready\/outdated/);
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
		// parseBody falls back to the raw string on a JSON.parse failure; a
		// handler reading `.variables` off that must not throw. `install` is
		// startable for POSTGRES ('ready'), so a 202 here proves the fallback
		// never reaches the router's own try/catch as a 500.
		const res = await mock.backend.fetch(`/v0/runtime/${enc(POSTGRES)}/install`, {
			method: 'POST',
			body: 'not json at all',
		});
		expect(res.status).toBe(202);
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
