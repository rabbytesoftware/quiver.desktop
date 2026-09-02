import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import type { DiscoveryJobDTO, DiscoveryJobStartedDTO, SearchResultDTO } from '@/lib/core-store/dtos/v0/search';
import { apiFetch } from '@/lib/transport/api';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { createMockBackend, type MockRuntime } from './index';
import { useMockStore } from './store';
import { buildWorld } from './world/build';
import { SCENARIOS } from './world/scenarios';
import { MOCK_HOST_PLATFORM, versioned } from './world/types';

const NS = 'github.com/rabbyte';
const MINECRAFT = `${NS}/minecraft@v1.21.4`;
const VALHEIM = `${NS}/valheim@v0.218.15`;
const POSTGRES = `${NS}/postgres@v17.2`;

let mock: MockRuntime;

function world() {
	return mock.world;
}

function get<T>(path: string): Promise<T> {
	return apiFetch<T>(path);
}

function post(path: string, body?: unknown): Promise<void> {
	return apiFetch<void>(path, {
		method: 'POST',
		...(body ? { body: JSON.stringify(body) } : {}),
	});
}

beforeEach(() => {
	useMockStore.setState({ latency: 0, errorRate: 0, unreachable: false });
	useMockStore.getState().resetFaults();
	mock = createMockBackend('normal');
	installBackend(mock.backend);
});

afterEach(() => {
	mock.dispose();
	resetBackend();
	vi.useRealTimers();
});

describe('routing', () => {
	it('answers /v0/health WITHOUT the envelope, the way core does', async () => {
		const res = await mock.backend.fetch('/v0/health');
		await expect(res.json()).resolves.toEqual({ status: 'ok' });
	});

	it('decodes a percent-encoded namespace containing / @ and .', async () => {
		const detail = await get<{ namespace: string; installed_ref: string }>(
			`/v0/arrow/${encodeURIComponent(MINECRAFT)}`
		);
		expect(detail.namespace).toBe(`${NS}/minecraft`);
		expect(detail.installed_ref).toBe('v1.21.4');
	});

	it('404s an unrouted path loudly rather than answering it empty', async () => {
		const res = await mock.backend.fetch('/v0/nonsense');
		expect(res.status).toBe(404);
		await expect(res.json()).resolves.toMatchObject({ success: false });
	});
});

describe('the catalog', () => {
	it('returns only library arrows when asked for user_installed', async () => {
		const all = await get<ArrowListResponseItemDTO[]>('/v0/arrow');
		const library = await get<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		expect(library.length).toBeLessThan(all.length);
		expect(library.some((a) => a.namespace === `${NS}/mariadb`)).toBe(false);
	});

	it('groups refs under versions rather than flattening them', async () => {
		const library = await get<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		const minecraft = library.find((a) => a.namespace === `${NS}/minecraft`);
		expect(minecraft?.namespace).not.toContain('@');
		expect(minecraft?.versions).toEqual([expect.objectContaining({ ref: 'v1.21.4', state: 'running' })]);
	});

	it('nests media where stable-26.5.1 puts it', async () => {
		const library = await get<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		const minecraft = library.find((a) => a.namespace === `${NS}/minecraft`);
		expect(minecraft?.media?.icon).toMatch(/^data:image\/svg\+xml,/);
	});
});

describe('the normal scenario', () => {
	it('puts all eleven arrow states on screen at once', async () => {
		const library = await get<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		const states = new Set(library.flatMap((a) => a.versions.map((v) => v.state)));
		expect(states).toEqual(
			new Set([
				'absent',
				'installing',
				'updating',
				'ready',
				'running',
				'stopping',
				'draining',
				'detached',
				'uninstalling',
				'removed',
				'outdated',
			])
		);
	});

	it('ships an arrow whose last install died partway, with the error on the step', async () => {
		const detail = await get<{
			last_return: { outcome: string; steps: Array<{ status: string; error?: string }> };
		}>(`/v0/arrow/${encodeURIComponent(`${NS}/factorio@v2.0.28`)}`);
		expect(detail.last_return.outcome).toBe('failed');
		const failed = detail.last_return.steps.find((s) => s.status === 'failed');
		expect(failed?.error).toMatch(/checksum mismatch/);
		expect(detail.last_return.steps.filter((s) => s.status === 'pending')).toHaveLength(2);
	});

	it('ships a collection member that does not resolve, with no name and no reason', async () => {
		const detail = await get<{ arrows: Array<{ namespace: string; resolved: boolean; name?: string }> }>(
			`/v0/collection/${encodeURIComponent(`${NS}/game-servers`)}`
		);
		const unresolved = detail.arrows.filter((a) => !a.resolved);
		expect(unresolved).toHaveLength(1);
		expect(unresolved[0].namespace).toMatch(/ark-survival/);
		expect(unresolved[0].name).toBeUndefined();
		expect(unresolved[0]).not.toHaveProperty('reason');
	});

	it('ships a package with no methods at all', async () => {
		// Methods live on the manifest endpoint, not the plain detail call --
		// matches quiver.core's real split (GET /v0/arrow/:ns carries no targets).
		const manifest = await get<{ targets: Record<string, { methods: Record<string, unknown> }> }>(
			`/v0/arrow/${encodeURIComponent(`${NS}/pixelmon-assets@v9.2.1`)}/manifest`
		);
		expect(Object.keys(Object.values(manifest.targets)[0].methods)).toHaveLength(0);
	});
});

describe('the readme endpoint', () => {
	it('serves the readme for an arrow that has one, from the bare namespace', async () => {
		const readme = await get<{ namespace: string; readme: string }>(
			`/v0/arrow/${encodeURIComponent(`${NS}/minecraft`)}/readme`
		);
		expect(readme.namespace).toBe(`${NS}/minecraft`);
		expect(readme.readme).toContain('Minecraft Server packages');
	});

	it('404s for an arrow with no readme', async () => {
		const res = await mock.backend.fetch(`/v0/arrow/${encodeURIComponent(`${NS}/redis`)}/readme`);
		expect(res.status).toBe(404);
	});

	it('400s when the namespace carries a ref, matching /manifest', async () => {
		const res = await mock.backend.fetch(`/v0/arrow/${encodeURIComponent(MINECRAFT)}/readme`);
		expect(res.status).toBe(400);
	});

	it('404s for a namespace that does not exist at all', async () => {
		const res = await mock.backend.fetch(`/v0/arrow/${encodeURIComponent(`${NS}/nonexistent`)}/readme`);
		expect(res.status).toBe(404);
	});
});

describe('the dependencies endpoint', () => {
	it('serves what an arrow declares it needs', async () => {
		const deps = await get<{ namespace: string; dependencies: { namespace: string; type: string }[] }>(
			`/v0/arrow/${encodeURIComponent(MINECRAFT)}/dependencies`
		);
		expect(deps.namespace).toBe(MINECRAFT);
		expect(deps.dependencies).toEqual([{ namespace: `${NS}/nats@v2.10.24`, type: 'tool' }]);
	});

	it('serves an empty list for an arrow that declares no dependencies', async () => {
		const deps = await get<{ dependencies: unknown[] }>(`/v0/arrow/${encodeURIComponent(POSTGRES)}/dependencies`);
		expect(deps.dependencies).toEqual([]);
	});

	it('404s for a namespace that does not exist at all', async () => {
		const res = await mock.backend.fetch(`/v0/arrow/${encodeURIComponent(`${NS}/nonexistent`)}/dependencies`);
		expect(res.status).toBe(404);
	});
});

describe('the dependents endpoint', () => {
	it('serves every arrow that declares a dependency on this one, computed from the reverse scan', async () => {
		const nats = `${NS}/nats@v2.10.24`;
		const dependents = await get<{ namespace: string; dependents: string[] }>(
			`/v0/arrow/${encodeURIComponent(nats)}/dependents`
		);
		expect(dependents.namespace).toBe(nats);
		expect(dependents.dependents).toEqual([MINECRAFT]);
	});

	it('serves an empty list for an arrow nothing depends on', async () => {
		const dependents = await get<{ dependents: unknown[] }>(
			`/v0/arrow/${encodeURIComponent(MINECRAFT)}/dependents`
		);
		expect(dependents.dependents).toEqual([]);
	});

	it('404s for a namespace that does not exist at all', async () => {
		const res = await mock.backend.fetch(`/v0/arrow/${encodeURIComponent(`${NS}/nonexistent`)}/dependents`);
		expect(res.status).toBe(404);
	});
});

describe('media fixtures', () => {
	// Inverted against the real daemon: nightly-96821f4 serves a published
	// arrow with an icon and an empty banner, so the icon is what every card
	// can count on. Requiring banners here is what kept the fallback the app
	// actually renders out of every test.
	it('gives every arrow the mock can serve an icon', () => {
		for (const scenario of SCENARIOS) {
			const data = scenario.build();
			const arrows = [...data.arrows, ...data.discoverable.map((c) => c.arrow)];
			for (const arrow of arrows) {
				expect(arrow.icon, `${scenario.name}: ${arrow.namespace} icon`).not.toBeNull();
			}
		}
	});

	it('keeps at least one banner in play, so that path stays covered', () => {
		const arrows = SCENARIOS.flatMap((s) => s.build().arrows);
		expect(arrows.some((a) => a.banner !== null)).toBe(true);
	});
});

describe('install', () => {
	beforeEach(() => vi.useFakeTimers());

	it('walks the timeline and lands on ready', async () => {
		const key = POSTGRES;
		const arrow = mock.world.arrows.get(key)!;
		arrow.state = 'absent';

		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);
		expect(arrow.state).toBe('installing');
		expect(arrow.active_run?.steps).toHaveLength(5);

		await vi.advanceTimersByTimeAsync(700);
		expect(arrow.active_run?.steps[0].status).toBe('running');

		await vi.advanceTimersByTimeAsync(700 * 5);
		expect(arrow.state).toBe('ready');
		expect(arrow.active_run).toBeNull();
		expect(arrow.last_return?.outcome).toBe('success');
		expect(arrow.last_return?.steps.every((s) => s.status === 'completed')).toBe(true);
	});

	it('pushes a runtime frame on every step, down the socket the app subscribes to', async () => {
		const socket = mock.backend.openSocket('/v0/runtime');
		const frames: unknown[] = [];
		socket.onmessage = (e) => frames.push(JSON.parse(e.data));
		await vi.advanceTimersByTimeAsync(0);

		const key = POSTGRES;
		mock.world.arrows.get(key)!.state = 'absent';
		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);
		await vi.advanceTimersByTimeAsync(700 * 6);

		expect(frames.length).toBeGreaterThan(5);
		expect(frames[0]).toMatchObject({ namespace: key, state: 'installing' });
		expect(frames[frames.length - 1]).toMatchObject({ namespace: key, state: 'ready' });
	});

	it('refuses with 422 when no target matches the host platform', async () => {
		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(VALHEIM)}/install`, { method: 'POST' });
		expect(res.status).toBe(422);
		await expect(res.json()).resolves.toMatchObject({
			error: expect.stringContaining(MOCK_HOST_PLATFORM),
		});
	});

	it('refuses a second action while one is already in flight', async () => {
		const key = POSTGRES;
		mock.world.arrows.get(key)!.state = 'absent';
		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);

		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(key)}/uninstall`, { method: 'POST' });
		expect(res.status).toBe(409);
	});

	it('stops mid-flight when the world is disposed, leaving no orphan timer', async () => {
		const key = POSTGRES;
		const arrow = mock.world.arrows.get(key)!;
		arrow.state = 'absent';
		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);
		await vi.advanceTimersByTimeAsync(700);

		mock.dispose();
		const frozen = arrow.state;
		await vi.advanceTimersByTimeAsync(700 * 10);

		expect(arrow.state).toBe(frozen);
		expect(arrow.state).toBe('installing');
	});
});

describe('methods', () => {
	beforeEach(() => vi.useFakeTimers());

	it('refuses a custom method that is not available in the current state', async () => {
		// 'rcon' is only available_in ['running'] on MINECRAFT's own manifest.
		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(MINECRAFT)}/rcon`, { method: 'POST' });
		expect(res.status).toBe(202);

		const arrow = mock.world.arrows.get(MINECRAFT)!;
		arrow.state = 'absent';
		const bad = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(MINECRAFT)}/rcon`, { method: 'POST' });
		expect(bad.status).toBe(409);
		await expect(bad.json()).resolves.toMatchObject({ error: expect.stringContaining('available in running') });
	});

	it('leaves a started arrow running, with a pid, via the reserved execute verb', async () => {
		const key = POSTGRES;
		mock.world.arrows.get(key)!.state = 'ready';
		await post(`/v0/runtime/${encodeURIComponent(key)}/execute`);
		await vi.advanceTimersByTimeAsync(700 * 5);

		const arrow = mock.world.arrows.get(key)!;
		expect(arrow.state).toBe('running');
		expect(arrow.active_run?.pid).toBeGreaterThan(0);
	});
});

describe('library membership', () => {
	it('adds and removes without deleting the arrow from the searchable world', async () => {
		const key = `${NS}/mariadb@v11.6.2`;
		await post(`/v0/arrow/${encodeURIComponent(key)}`);
		expect(mock.world.arrows.get(key)?.user_installed).toBe(true);

		await apiFetch(`/v0/arrow/${encodeURIComponent(key)}`, { method: 'DELETE' });
		expect(mock.world.arrows.get(key)?.user_installed).toBe(false);
		// Search reports the bare namespace; the refs travel in versions.
		const hits = await get<SearchResultDTO[]>('/v0/search?q=mariadb');
		const hit = hits.find((h) => h.namespace === key.split('@')[0]);
		expect(hit?.versions).toContain(key.split('@')[1]);
	});
});

describe('search', () => {
	it('rejects an empty q the way core does, rather than answering with everything', async () => {
		const res = await mock.backend.fetch('/v0/search?q=%20%20');
		expect(res.status).toBe(400);
		await expect(res.json()).resolves.toMatchObject({ success: false });
	});

	it('returns one row per namespace, carrying every local ref', async () => {
		const hits = await get<SearchResultDTO[]>('/v0/search?q=minecraft');
		const hit = hits.find((h) => h.namespace === `${NS}/minecraft`);
		expect(hit).toBeDefined();
		expect(hit!.namespace).not.toContain('@');
		expect(hit!.versions).toContain('v1.21.4');
	});

	it('reports every catalog hit as installed and known', async () => {
		const hits = await get<SearchResultDTO[]>('/v0/search?q=minecraft');
		expect(hits.every((h) => h.installed && h.known)).toBe(true);
	});

	it('caps limit at 100 and defaults it to 25', async () => {
		// 'normal' holds 17 arrows, so every bound would pass vacuously there.
		const big = createMockBackend('extreme');
		installBackend(big.backend);
		try {
			expect((await get<SearchResultDTO[]>('/v0/search?q=quiver-demo')).length).toBe(25);
			expect((await get<SearchResultDTO[]>('/v0/search?q=quiver-demo&limit=500')).length).toBe(100);
			expect((await get<SearchResultDTO[]>('/v0/search?q=quiver-demo&limit=7')).length).toBe(7);
		} finally {
			big.dispose();
			installBackend(mock.backend);
		}
	});
});

describe('discovery', () => {
	beforeEach(() => vi.useFakeTimers());

	function start(q: string): Promise<DiscoveryJobStartedDTO> {
		return apiFetch<DiscoveryJobStartedDTO>('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q }),
		});
	}

	it('answers with a ticket before any provider has been asked', async () => {
		const started = await start('server');
		expect(started.job_id).toBeTruthy();
		expect(started.query).toBe('server');
		expect(Date.parse(started.expires_at)).not.toBeNaN();
		expect(started).not.toHaveProperty('providers');
		expect(started).not.toHaveProperty('results');
	});

	it('reports zeroes and no providers while the pass is still running', async () => {
		const started = await start('server');
		const mid = await get<DiscoveryJobDTO>(`/v0/search/discover/${started.job_id}`);
		expect(mid.status).toBe('running');
		expect(mid).toMatchObject({ found: 0, verified: 0, skipped: 0, providers: [] });
	});

	it('streams each result over the job socket rather than in the summary', async () => {
		const started = await start('server');
		const socket = mock.backend.openSocket(`/v0/search/discover/${started.job_id}`);
		const frames: SearchResultDTO[] = [];
		socket.onmessage = (e) => frames.push(JSON.parse(e.data));

		await vi.advanceTimersByTimeAsync(5000);

		expect(frames.length).toBeGreaterThan(0);
		const summary = await get<DiscoveryJobDTO>(`/v0/search/discover/${started.job_id}`);
		expect(summary).not.toHaveProperty('results');
		expect(summary.verified).toBe(frames.length);
	});

	it('arrives over time rather than all at once', async () => {
		const started = await start('server');
		const socket = mock.backend.openSocket(`/v0/search/discover/${started.job_id}`);
		let count = 0;
		socket.onmessage = () => count++;

		await vi.advanceTimersByTimeAsync(300);
		const early = count;
		await vi.advanceTimersByTimeAsync(5000);

		expect(early).toBeGreaterThan(0);
		expect(count).toBeGreaterThan(early);
	});

	it('streams arrows the catalog does not hold, as not installed', async () => {
		const started = await start('server');
		const socket = mock.backend.openSocket(`/v0/search/discover/${started.job_id}`);
		const frames: SearchResultDTO[] = [];
		socket.onmessage = (e) => frames.push(JSON.parse(e.data));

		await vi.advanceTimersByTimeAsync(5000);

		const fresh = frames.filter((f) => !f.installed);
		expect(fresh.length).toBeGreaterThan(0);
		expect(fresh.every((f) => f.provenance === 'seen')).toBe(true);
		expect(fresh.every((f) => !world().arrows.has(`${f.namespace}@${f.versions[0]}`))).toBe(true);
	});

	it('reports a rediscovered arrow as known but still not installed', async () => {
		const first = await start('server');
		mock.backend.openSocket(`/v0/search/discover/${first.job_id}`);
		await vi.advanceTimersByTimeAsync(5000);

		const second = await start('server');
		const socket = mock.backend.openSocket(`/v0/search/discover/${second.job_id}`);
		const frames: SearchResultDTO[] = [];
		socket.onmessage = (e) => frames.push(JSON.parse(e.data));
		await vi.advanceTimersByTimeAsync(5000);

		const seen = frames.filter((f) => !f.installed);
		expect(seen.length).toBeGreaterThan(0);
		expect(seen.every((f) => f.known)).toBe(true);
	});

	it('reports a rate-limited provider with a retry-after rather than as no results', async () => {
		const started = await start('server');
		mock.backend.openSocket(`/v0/search/discover/${started.job_id}`);
		await vi.advanceTimersByTimeAsync(5000);

		const done = await get<DiscoveryJobDTO>(`/v0/search/discover/${started.job_id}`);

		expect(done.status).toBe('completed');
		const refused = done.providers.find((p) => !p.ok);
		expect(refused).toMatchObject({ host: 'gitlab.com', reason: 'rate limited', retry_after: 40 });
		expect(refused!.returned).toBe(0);

		// A refusal is not an empty result: the host that did answer still
		// reports what it returned, and found/verified/skipped add up.
		const answered = done.providers.find((p) => p.ok)!;
		expect(answered.returned).toBeGreaterThan(0);
		expect(done.found).toBe(answered.returned);
		expect(done.verified + done.skipped).toBe(done.found);
		expect(done.skipped).toBeGreaterThan(0);
	});
});

describe('config', () => {
	it('serves the search settings the inspector reads', async () => {
		const cfg = await get<{ running: { search: Record<string, unknown> }; restart_required: string[] }>(
			'/v0/config'
		);
		expect(cfg.running.search).toMatchObject({
			per_provider_limit: expect.any(Number),
			fetch_concurrency: expect.any(Number),
			provider_timeout: expect.any(String),
		});
		expect(Array.isArray(cfg.restart_required)).toBe(true);
	});
});

describe('determinism', () => {
	it('builds the extreme world identically twice', () => {
		const noEmitter = { emit: () => {} };
		const a = buildWorld('extreme', noEmitter);
		const b = buildWorld('extreme', noEmitter);
		expect([...a.arrows.keys()]).toEqual([...b.arrows.keys()]);
		expect([...a.arrows.values()].map((x) => x.state)).toEqual([...b.arrows.values()].map((x) => x.state));
		a.clock.cancelAll();
		b.clock.cancelAll();
	});

	it('gives every scenario its own cache partition', () => {
		const noEmitter = { emit: () => {} };
		expect(buildWorld('normal', noEmitter).connectionId).toBe('mock:normal');
		expect(buildWorld('empty', noEmitter).connectionId).toBe('mock:empty');
	});

	it('does not share mutable fixtures between two worlds', async () => {
		const noEmitter = { emit: () => {} };
		const first = buildWorld('normal', noEmitter);
		first.arrows.get(MINECRAFT)!.state = 'removed';
		const second = buildWorld('normal', noEmitter);
		expect(second.arrows.get(MINECRAFT)!.state).toBe('running');
		first.clock.cancelAll();
		second.clock.cancelAll();
	});
});

describe('the empty scenario', () => {
	it('has nothing in it', async () => {
		mock.dispose();
		mock = createMockBackend('empty');
		installBackend(mock.backend);
		await expect(get<unknown[]>('/v0/arrow?user_installed=true')).resolves.toEqual([]);
		await expect(get<unknown[]>('/v0/collection')).resolves.toEqual([]);
	});
});

describe('chaos', () => {
	it('forces a route to fail when its fault is turned all the way up', async () => {
		useMockStore.getState().setFault('arrows', 100);
		const res = await mock.backend.fetch('/v0/arrow');
		expect(res.status).toBe(500);
		await expect(res.json()).resolves.toMatchObject({ error: expect.stringContaining('arrows') });
	});

	it('leaves every other route alone', async () => {
		useMockStore.getState().setFault('search', 100);
		const res = await mock.backend.fetch('/v0/arrow');
		expect(res.status).toBe(200);
	});

	it('marks an unreachable daemon the way the Rust proxy does, and apiFetch retries it', async () => {
		useMockStore.getState().setUnreachable(true);

		const raw = await mock.backend.fetch('/v0/arrow');
		expect(raw.status).toBe(502);
		expect(raw.headers.get('x-quiver-proxy')).toBe('error');

		const sleep = vi.fn().mockResolvedValue(undefined);
		await expect(
			apiFetch('/v0/arrow', undefined, { attempts: 4, baseDelayMs: 0, maxDelayMs: 0, sleep })
		).rejects.toThrow(/unreachable/);
		expect(sleep).toHaveBeenCalledTimes(3);
	});

	it('does not retry a mutation, however unreachable the daemon is', async () => {
		useMockStore.getState().setUnreachable(true);
		const sleep = vi.fn().mockResolvedValue(undefined);
		await expect(
			apiFetch('/v0/arrow/x', { method: 'POST' }, { attempts: 4, baseDelayMs: 0, maxDelayMs: 0, sleep })
		).rejects.toThrow();
		expect(sleep).not.toHaveBeenCalled();
	});
});

describe('the shell it impersonates', () => {
	it('reports the local mock connection, named so it cannot be mistaken for a real host', async () => {
		const { connections, active_id } = await mock.backend.getConnections();
		expect(active_id).toBe('mock:normal');
		expect(connections[0].name).toBe('Mock · Normal');
	});

	// The Remote Control screen has nothing to show off with only "Local" in
	// play -- a saved-but-inactive remote is what exercises that screen's
	// list, switcher, and empty-state-vs-populated paths under the mock.
	it('also registers one saved remote, so Remote Control has something to show', async () => {
		const { connections, active_id } = await mock.backend.getConnections();
		expect(connections).toHaveLength(2);
		const remote = connections.find((c) => c.kind === 'remote');
		expect(remote).toMatchObject({ kind: 'remote', id: 'mock:home-lab' });
		expect(remote?.name).toMatch(/^Mock · /);
		expect(remote?.url).toBeTruthy();
		expect(active_id).not.toBe(remote?.id);
	});

	it('announces starting before ready, so the connecting screen is reachable', async () => {
		vi.useFakeTimers();
		const seen: string[] = [];
		await mock.backend.onCoreStatus((s) => seen.push(s));
		await vi.advanceTimersByTimeAsync(500);
		expect(seen).toEqual(['starting', 'ready']);
	});
});

describe('versioned()', () => {
	it('joins the base namespace and the ref the way the store keys on', () => {
		expect(versioned({ namespace: 'github.com/a/b', ref: 'v1' })).toBe('github.com/a/b@v1');
	});
});

/**
 * The mock is only useful if it refuses what core refuses. A client bug that
 * these let through would pass every test here and fail against the daemon.
 */
describe('what discover refuses', () => {
	beforeEach(() => vi.useFakeTimers());

	it('rejects a POST with no body at all, not just one with an empty q', async () => {
		const res = await mock.backend.fetch('/v0/search/discover', { method: 'POST' });
		expect(res.status).toBe(400);
		await expect(res.json()).resolves.toMatchObject({ success: false });
	});

	it('rejects `query` in place of `q`, which core reads by name', async () => {
		const res = await mock.backend.fetch('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ query: 'server' }),
		});
		expect(res.status).toBe(400);
	});

	it('rejects a q that is only whitespace', async () => {
		const res = await mock.backend.fetch('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q: '   ' }),
		});
		expect(res.status).toBe(400);
	});

	it('survives a job deleted while its results are still landing', async () => {
		const started = await apiFetch<DiscoveryJobStartedDTO>('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q: 'server' }),
		});

		// The daemon's grace window can drop a job mid-pass; the staggered
		// emitters and the close callback both have to find it gone.
		world().jobs.delete(started.job_id);
		await vi.advanceTimersByTimeAsync(30_000);

		const res = await mock.backend.fetch(`/v0/search/discover/${started.job_id}`);
		expect(res.status).toBe(404);
	});
});

describe('the vault half of Lane A', () => {
	it('ignores a vault namespace nothing discoverable can account for', async () => {
		// The vault index can name a namespace whose manifest is no longer
		// reachable. It is not a result -- there is nothing to describe.
		world().vault.add('github.com/ghost/gone');

		const hits = await get<SearchResultDTO[]>('/v0/search?q=ghost');

		expect(hits.every((h) => h.namespace !== 'github.com/ghost/gone')).toBe(true);
	});
});
