import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import type { ResolvedReleaseAsset } from '@/domain/release';

import { QuiverWebSocket } from './quiver-socket';

export const SOCKET_OPEN = 1;

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
	fetch(path: string, init?: RequestInit): Promise<Response>;
	openSocket(path: string): SocketLike;
	/**
	 * The `stable-*` tag THIS binary was built from, or `null` for any build
	 * that was not cut from one (every dev, PR-CI and locally built app).
	 *
	 * A compile-time constant baked in by `src-tauri/build.rs`, not a runtime
	 * lookup and not `tauri.conf.json`'s `version` -- that is a productVersion
	 * ("0.1.0"), and no git ref will ever carry that name. See
	 * `src-tauri/src/commands/build_info.rs`.
	 */
	getBuildTag(): Promise<string | null>;
	/**
	 * This app's own newest release asset, for the machine it is running on.
	 *
	 * Native-side for the same reason `getBuildTag` is: selecting an asset
	 * needs the real OS and CPU architecture, and the webview cannot report
	 * either honestly (`navigator.platform` answers `MacIntel` on Apple
	 * Silicon). Rejects with a `ReleaseResolveError` -- see
	 * `src-tauri/src/release/mod.rs`.
	 */
	resolveReleaseAsset(): Promise<ResolvedReleaseAsset>;
	getConnections(): Promise<ConnectionsSnapshot>;
	onCoreStatus(cb: (status: ConnectionStatus) => void): Promise<() => void>;
	onConnectionsChanged(cb: (snapshot: ConnectionsSnapshot) => void): Promise<() => void>;
}

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

	getBuildTag() {
		return invoke<string | null>('get_build_tag');
	},

	resolveReleaseAsset() {
		return invoke<ResolvedReleaseAsset>('resolve_release_asset');
	},

	getConnections() {
		return invoke<ConnectionsSnapshot>('get_connections');
	},

	onCoreStatus(cb) {
		return listen<{ status: ConnectionStatus }>('core://status', (e) => cb(e.payload.status));
	},

	onConnectionsChanged(cb) {
		return listen<ConnectionsSnapshot>('connection://changed', (e) => cb(e.payload));
	},
};

let active: Backend = realBackend;

export function backend(): Backend {
	return active;
}

export function installBackend(next: Backend): void {
	active = next;
}

export function resetBackend(): void {
	active = realBackend;
}
