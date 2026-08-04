import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import { apiFetch } from '@/lib/transport/api';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { createMockBackend, type MockRuntime } from './index';
import { useMockStore } from './store';
import { buildWorld } from './world/build';
import { MOCK_HOST_PLATFORM, versioned } from './world/types';

const NS = 'github.com/rabbyte';
const MINECRAFT = `${NS}/minecraft@v1.21.4`;
const VALHEIM = `${NS}/valheim@v0.218.15`;
const POSTGRES = `${NS}/postgres@v17.2`;

let mock: MockRuntime;

/** Reads through the mock the way the app does — envelope unwrapped by apiFetch. */
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

	// Namespaces contain `/`, `@` and `.`, and every caller encodes them. If the
	// router did not decode, every arrow detail page would 404.
	it('decodes a percent-encoded namespace containing / @ and .', async () => {
		const detail = await get<{ namespace: string; installed_ref: string }>(
			`/v0/arrow/${encodeURIComponent(MINECRAFT)}`
		);
		expect(detail.namespace).toBe(`${NS}/minecraft`);
		expect(detail.installed_ref).toBe('v1.21.4');
	});

	// An unrouted path answered with an empty 200 would render as "you have no
	// arrows" — a plausible screen that is a lie.
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

	// `toArrowCatalogRecords` joins namespace and ref itself. A flat list keyed
	// by versioned namespace would come out as `github.com/x/y@v1@v1`.
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

	it('ships a collection member that does not resolve, with a reason', async () => {
		const detail = await get<{ arrows: Array<{ resolved: boolean; reason?: string }> }>(
			`/v0/collection/${encodeURIComponent(`${NS}/game-servers`)}`
		);
		const unresolved = detail.arrows.filter((a) => !a.resolved);
		expect(unresolved).toHaveLength(1);
		expect(unresolved[0].reason).toMatch(/yanked/);
	});

	// A package has no methods, so nothing may offer a Run button for it.
	it('ships a package with no methods at all', async () => {
		const detail = await get<{ targets: Array<{ methods: Record<string, unknown> }> }>(
			`/v0/arrow/${encodeURIComponent(`${NS}/pixelmon-assets@v9.2.1`)}`
		);
		expect(Object.keys(detail.targets[0].methods)).toHaveLength(0);
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
		await vi.advanceTimersByTimeAsync(0); // let the socket finish opening

		const key = POSTGRES;
		mock.world.arrows.get(key)!.state = 'absent';
		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);
		await vi.advanceTimersByTimeAsync(700 * 6);

		expect(frames.length).toBeGreaterThan(5);
		expect(frames[0]).toMatchObject({ namespace: key, state: 'installing' });
		expect(frames[frames.length - 1]).toMatchObject({ namespace: key, state: 'ready' });
	});

	// The Valheim fixture's whole reason for existing: the arrow page must be
	// able to say "no build for your platform" BEFORE the button hits this.
	it('refuses with 422 when no target matches the host platform', async () => {
		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(VALHEIM)}/install`, { method: 'POST' });
		expect(res.status).toBe(422);
		await expect(res.json()).resolves.toMatchObject({
			error: expect.stringContaining(MOCK_HOST_PLATFORM),
		});
	});

	// Core has no cancel, which is why the UI offers nothing in a transitional state.
	it('refuses a second action while one is already in flight', async () => {
		const key = POSTGRES;
		mock.world.arrows.get(key)!.state = 'absent';
		await post(`/v0/runtime/${encodeURIComponent(key)}/install`);

		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(key)}/uninstall`, { method: 'POST' });
		expect(res.status).toBe(409);
	});

	// Disposal is what a scenario switch and a page teardown both do.
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

	it('refuses a method that is not available in the current state', async () => {
		// `rcon` is available_in ['running']; postgres is ready.
		const res = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(MINECRAFT)}/_execute`, {
			method: 'POST',
			body: JSON.stringify({ method: 'stop' }),
		});
		// minecraft IS running, so stop is allowed — assert the inverse case.
		expect(res.status).toBe(202);

		const bad = await mock.backend.fetch(`/v0/runtime/${encodeURIComponent(POSTGRES)}/_execute`, {
			method: 'POST',
			body: JSON.stringify({ method: 'stop' }),
		});
		expect(bad.status).toBe(409);
		await expect(bad.json()).resolves.toMatchObject({ error: expect.stringContaining('available in running') });
	});

	it('leaves a started arrow running, with a pid', async () => {
		const key = POSTGRES;
		await post(`/v0/runtime/${encodeURIComponent(key)}/_execute`, { method: 'start' });
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
		// Still findable — leaving the library is not ceasing to exist.
		const hits = await get<Array<{ namespace: string }>>('/v0/search?q=mariadb');
		expect(hits.map((h) => h.namespace)).toContain(key);
	});
});

describe('discovery', () => {
	beforeEach(() => vi.useFakeTimers());

	// The single most misleading thing a search screen can do is render "a host refused"
	// as "there is nothing called that".
	it('reports a rate-limited provider with a retry-after rather than as no results', async () => {
		const started = await apiFetch<{ id: string; status: string }>('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q: 'server' }),
		});
		expect(started.status).toBe('running');

		await vi.advanceTimersByTimeAsync(1500);

		const done = await get<{
			status: string;
			providers: Array<{ host: string; ok: boolean; reason?: string; retry_after?: number }>;
		}>(`/v0/search/discover/${started.id}`);

		expect(done.status).toBe('done');
		const refused = done.providers.find((p) => !p.ok);
		expect(refused).toMatchObject({ host: 'gitlab.com', reason: 'rate limited', retry_after: 40 });
		expect(done.providers.some((p) => p.ok)).toBe(true);
	});
});

describe('determinism', () => {
	// Scenarios must build byte-identically every run, or screenshots drift and
	// a random source could creep in unnoticed.
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

	// The one fault that is not a probability, and the only one that exercises apiFetch's
	// retry ladder: a proxy-marked 502 is what a refused socket actually looks like under
	// a URI-scheme proxy.
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
	it('reports one connection, named so it cannot be mistaken for a real host', async () => {
		const { connections, active_id } = await mock.backend.getConnections();
		expect(connections).toHaveLength(1);
		expect(active_id).toBe('mock:normal');
		expect(connections[0].name).toBe('Mock · Normal');
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
