import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';

import { QuiverWebSocket } from './quiver-socket';

/** `WebSocket.OPEN`. Restated so `wsManager` need not know the implementation. */
export const SOCKET_OPEN = 1;

/** The subset of `WebSocket` that `wsManager` drives. */
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
	 * MAY THROW SYNCHRONOUSLY when it has no origin to dial, and callers depend
	 * on it: a broken shell must stay distinguishable from a daemon that is
	 * down. See `coreIsReachable`, which catches the second and not the first.
	 */
	fetch(path: string, init?: RequestInit): Promise<Response>;
	openSocket(path: string): SocketLike;
	getConnections(): Promise<ConnectionsSnapshot>;
	onCoreStatus(cb: (status: ConnectionStatus) => void): Promise<() => void>;
	onConnectionsChanged(cb: (snapshot: ConnectionsSnapshot) => void): Promise<() => void>;
}

/**
 * The origin the shell injected, with no fallback literal.
 *
 * WebView2 cannot register a non-standard scheme, so wry serves
 * `http://quiver.localhost` on Windows and `quiver://localhost` on the two
 * WebKit platforms. Any literal here is silently right on two and wrong on the
 * third, and no suite that runs on macOS and Linux would catch it — so the
 * shell computes it per platform (`QUIVER_API_BASE`, src-tauri/src/lib.rs) and
 * this takes what it is given, or throws.
 */
export function apiBase(): string {
	const base = (window as unknown as { __QUIVER__?: { api?: string } }).__QUIVER__?.api;
	if (base) return base;
	throw new Error(
		'window.__QUIVER__.api is not set, so there is no API origin to dial. The page is either not running ' +
			'inside the Quiver shell (which injects a per-platform origin at document-start) or that injection failed.'
	);
}

export const realBackend: Backend = {
	fetch(path, init) {
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
		// Returns `cb`'s promise rather than discarding it: the core-store
		// listener is async and its callers await the emit to know the stream
		// restart finished.
		return listen<{ status: ConnectionStatus }>('core://status', (e) => cb(e.payload.status));
	},

	onConnectionsChanged(cb) {
		return listen<ConnectionsSnapshot>('connection://changed', (e) => cb(e.payload));
	},
};

let active: Backend = realBackend;

/**
 * Resolved per call, but only ever changed at boot — before `setupListeners`
 * and before React renders. A mid-session swap would leave live sockets and a
 * seeded cache belonging to the old backend, which is why turning the mock on
 * reloads.
 */
export function backend(): Backend {
	return active;
}

export function installBackend(next: Backend): void {
	active = next;
}

/** Tests only. */
export function resetBackend(): void {
	active = realBackend;
}
