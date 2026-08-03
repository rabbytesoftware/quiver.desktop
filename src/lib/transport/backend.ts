// The four ways this app touches the world outside the webview, named once.
//
// They were reached three different ways — `fetch` over the `quiver://` scheme
// proxy, `QuiverWebSocket` over a Tauri Channel, and `invoke`/`listen` for the
// connection list and core status — so "talk to the daemon" had no single
// definition and therefore no single place to substitute. That mattered the
// moment we wanted to develop the UI without a daemon at all: there was nothing
// to stand in for, only three unrelated call shapes scattered across four
// modules.
//
// This is not an abstraction over HTTP. It is an abstraction over WHERE THE
// DAEMON IS, and the set is closed: adding a fifth method means the app learned
// a new way to reach outside itself, which is worth noticing.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';

import { QuiverWebSocket } from './quiver-socket';

/**
 * `WebSocket.OPEN`, restated.
 *
 * `wsManager` needs to ask "is this socket writable" without knowing which
 * implementation it holds, and reading the constant off a concrete class would
 * re-couple it to one. The value is fixed by the WebSocket standard, so every
 * conforming implementation — the real shim, the mock, and the test fake —
 * already agrees on it.
 */
export const SOCKET_OPEN = 1;

/**
 * The subset of `WebSocket` that `wsManager` drives.
 *
 * Deliberately not `WebSocket` itself: the real transport is not one (see
 * `quiver-socket.ts` — the browser constructor cannot reach a unix socket), and
 * a type demanding `binaryType`, `bufferedAmount` or `addEventListener` would
 * force every implementation to grow members nothing calls.
 */
export interface SocketLike {
	readyState: number;
	onopen: (() => void) | null;
	onmessage: ((event: { data: string }) => void) | null;
	onclose: (() => void) | null;
	onerror: ((event: unknown) => void) | null;
	send(data: string): void;
	close(): void;
}

export interface ConnectionsSnapshot {
	connections: ConnectionConfig[];
	active_id: string;
}

export interface Backend {
	/**
	 * Forward one request. `path` is a leading-slash route (`/v0/arrow`); the
	 * backend owns the origin, because only the backend knows whether there is
	 * one.
	 *
	 * MAY THROW SYNCHRONOUSLY, and callers depend on it. A backend with no
	 * origin to dial is a broken shell, not a daemon that is down, and the two
	 * must stay distinguishable: `coreIsReachable` catches the second and
	 * deliberately does not catch the first. A rejected promise would collapse
	 * them into one.
	 */
	fetch(path: string, init?: RequestInit): Promise<Response>;

	/** Dial `path` and return a socket that is CONNECTING until it opens. */
	openSocket(path: string): SocketLike;

	getConnections(): Promise<ConnectionsSnapshot>;

	onCoreStatus(cb: (status: ConnectionStatus) => void): Promise<() => void>;

	onConnectionsChanged(cb: (snapshot: ConnectionsSnapshot) => void): Promise<() => void>;
}

/**
 * The origin every request is sent to — whatever the shell injected, and
 * nothing else.
 *
 * There is deliberately NO fallback literal. The reachable origin is not the
 * same on every platform: WebView2 cannot register a non-standard scheme, so
 * wry registers `http://quiver.localhost` and rewrites requests back before
 * handing them to the handler, while WKWebView and WebKitGTK serve
 * `quiver://localhost` as written. Any literal here is therefore silently
 * correct on two platforms and silently wrong on the third, and nothing can
 * see it: it is a string, `tsc` has no opinion on it, and a suite that runs on
 * macOS and Linux never dials the origin Windows needs. So the shell computes
 * it once per platform (`QUIVER_API_BASE`, src-tauri/src/lib.rs) and injects it
 * at document-start; this module takes what it is given.
 *
 * Absent config means one of two things, and neither has a correct origin to
 * guess at: the page is not inside the Tauri shell (a bare `vite` dev server,
 * which cannot reach the daemon by any URL), or the init script did not run.
 * Both are diagnosable the moment they say so, and mysterious the moment they
 * are papered over with a plausible-looking default — a wrong base fails as an
 * ordinary network error several layers away from its cause. So it throws.
 *
 * Resolved per call rather than at module load, so that merely IMPORTING this
 * module outside the shell stays harmless — the test suite does exactly that,
 * every time it mocks `apiFetch` — and the failure lands on the caller that
 * actually wanted the network.
 *
 * Note the one case this is NOT an error: the mock backend, which genuinely has
 * no origin and never calls this. "No origin" is only a fault for a backend
 * that needs one.
 */
export function apiBase(): string {
	const base = (window as unknown as { __QUIVER__?: { api?: string } }).__QUIVER__?.api;
	if (base) return base;
	throw new Error(
		'window.__QUIVER__.api is not set, so there is no API origin to dial. The page is either not running ' +
			'inside the Quiver shell (which injects a per-platform origin at document-start) or that injection failed.'
	);
}

/** What has always run: the `quiver://` proxy, the Rust WS bridge, Tauri IPC. */
export const realBackend: Backend = {
	fetch(path, init) {
		// Resolved here rather than inside the returned promise, so a missing
		// origin throws synchronously — see the `Backend.fetch` doc comment.
		const base = apiBase();
		return globalThis.fetch(`${base}${path}`, init);
	},

	openSocket(path) {
		return new QuiverWebSocket(path);
	},

	getConnections() {
		return invoke<ConnectionsSnapshot>('get_connections');
	},

	onCoreStatus(cb) {
		// The handler RETURNS what `cb` returns, rather than discarding it. The
		// core-store listener is async and its callers await the emit to know the
		// stream restart finished; swallowing the promise here would make every
		// such wait resolve one tick early, against streams that had not started.
		return listen<{ status: ConnectionStatus }>('core://status', (e) => cb(e.payload.status));
	},

	onConnectionsChanged(cb) {
		return listen<ConnectionsSnapshot>('connection://changed', (e) => cb(e.payload));
	},
};

// Module-level rather than passed down: every consumer would otherwise need a
// backend threaded through it, including modules with no other reason to know
// one exists (`entity-stream`, the persistence layer). The cost is a mutable
// global, bounded by the rule below.
let active: Backend = realBackend;

/**
 * Which backend the app is talking to.
 *
 * Resolved per call, but the answer is only ever CHANGED at boot, before
 * `setupListeners` runs and before React renders. A swap mid-session would
 * leave live sockets and an already-seeded cache belonging to the old backend,
 * which is why turning the mock on reloads the page instead.
 */
export function backend(): Backend {
	return active;
}

export function installBackend(next: Backend): void {
	active = next;
}

/** Tests only — restores the default so one suite cannot leak into the next. */
export function resetBackend(): void {
	active = realBackend;
}
