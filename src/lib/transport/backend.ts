import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import type { ResolvedReleaseAsset } from '@/domain/release';

import { startConsoleExec, type ConsoleRun } from './console-exec';
import { QuiverWebSocket } from './quiver-socket';

export type { ConsoleRun } from './console-exec';

export const SOCKET_OPEN = 1;

/** What the daemon said when it closed a stream; absent when the transport could not tell. */
export interface SocketCloseInfo {
	code: number;
	reason: string;
}

export interface SocketLike {
	readyState: number;
	onopen: (() => void) | null;
	onmessage: ((event: { data: string }) => void) | null;
	onclose: ((info?: SocketCloseInfo) => void) | null;
	onerror: ((event: unknown) => void) | null;
	send(data: string): void;
	close(): void;
}

export interface ConnectionsSnapshot {
	connections: ConnectionConfig[];
	active_id: string;
}

/**
 * What `src-tauri/build.rs` baked into this binary for the build indicator
 * (`commands::build_info::BuildStamp`). Every field is null for a build that was
 * not stamped.
 */
export interface BuildStamp {
	/** The full 40-character commit. */
	commit: string | null;
	/** Unix seconds. */
	built_at: number | null;
	/** The release tag this build is published as, in any channel. */
	label: string | null;
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
	/** The stamps the build indicator shows for this binary; see {@link BuildStamp}. */
	getBuildStamp(): Promise<BuildStamp>;
	/**
	 * Runs `line` on the active daemon's console and streams its frames to
	 * `onFrame` -- one NDJSON line each, ending in an `exit` or `error` frame
	 * (`docs/console-spec.md`). Native-side because the `quiver://` proxy
	 * returns a response whole and cannot carry a stream.
	 *
	 * The line is sent exactly as typed: the daemon owns the grammar.
	 */
	execConsole(line: string, onFrame: (frame: string) => void): ConsoleRun;
	/**
	 * This machine's real `"os/arch"` platform key -- e.g. `"darwin/arm64"` --
	 * the way quiver.core's manifest `targets` are keyed.
	 *
	 * Native-side, not `src/lib/platform.ts`'s `currentPlatform()` UA guess:
	 * `std::env::consts::OS`/`ARCH` are the compiler's own honest answer for
	 * the binary actually running, so this can't misdetect an Apple Silicon
	 * Mac's architecture the way the webview guess can. See
	 * `src-tauri/src/commands/platform.rs`.
	 */
	getPlatform(): Promise<string>;
	/**
	 * This app's own release asset from the release tagged `tag` -- the newest
	 * release when `tag` is undefined -- for the machine it is running on.
	 *
	 * Native-side for the same reason `getBuildTag` is: selecting an asset
	 * needs the real OS and CPU architecture, and the webview cannot report
	 * either honestly (`navigator.platform` answers `MacIntel` on Apple
	 * Silicon). Rejects with a `ReleaseResolveError` -- see
	 * `src-tauri/src/release/mod.rs`.
	 */
	resolveReleaseAsset(tag?: string): Promise<ResolvedReleaseAsset>;
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

	getBuildStamp() {
		return invoke<BuildStamp>('get_build_stamp');
	},

	execConsole(line, onFrame) {
		return startConsoleExec(line, onFrame);
	},

	getPlatform() {
		return invoke<string>('get_platform');
	},

	resolveReleaseAsset(tag) {
		return invoke<ResolvedReleaseAsset>('resolve_release_asset', { tag: tag ?? null });
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
