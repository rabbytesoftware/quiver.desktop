import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch } from '@/lib/transport/api';
import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import type { ArrowListResponseItemDTO } from '../dtos/v0/arrow';
import { toArrowCatalogRecords, toInitialRuntimeUpdates } from '../dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

const RUNTIME_ENDPOINT = '/v0/runtime';

interface GetConnectionsResponse {
	connections: ConnectionConfig[];
	active_id: string;
}

/**
 * Only two Tauri events survive the data-layer rewrite: `core://status` and
 * `connection://changed`. Both are native lifecycle facts the webview cannot
 * observe on its own. Everything else — the arrow catalog and the runtime
 * overlay — now rides the transport layer (`apiFetch` + `wsManager`) instead
 * of the bespoke `arrow://event` / `runtime://update` events, which no
 * longer exist.
 *
 * `connection://changed` is registered (add/remove/rename still emit it —
 * `@/lib/connection/listeners.ts` is what consumes it for the connections
 * UI) but this module does NOT use it to learn which connection is active:
 * `switch_connection` does not currently emit it at all (a known Rust-side
 * gap — `manager.rs` awaits `guard.start(app)`, which emits `starting` then
 * `ready` before the command returns, so even an appended emit would land
 * after the `ready` that already restarted the streams). Instead, `ready`
 * below calls `get_connections` directly, which makes it self-sufficient
 * and immune to that ordering/emission gap.
 */
export async function setupListeners(): Promise<void> {
	// Must run before any seed touches the cache (design decision #4).
	await maybeWipeOnVersionChange();

	// Local to this call, not module-level: setupListeners runs exactly once
	// per app lifetime in production, so a closure gives every listener a
	// consistent view of "which live subscriptions" without leaking state
	// across repeated calls (e.g. in tests).
	let disposeArrowStream: (() => void) | null = null;
	let disposeRuntimeStream: (() => void) | null = null;

	function stopStreams(): void {
		disposeArrowStream?.();
		disposeArrowStream = null;
		disposeRuntimeStream?.();
		disposeRuntimeStream = null;
	}

	function startStreams(connectionId: string): void {
		// Captured by `seed` below, consumed once by the `onChange` that
		// immediately follows it. Neither `/v0/arrow` nor `/v0/runtime` push
		// anything on connect (both are transition-only sockets, verified
		// directly against stable-26.5.1) — so the seed GET's own
		// `versions[].state` is the ONLY source for a correct initial paint.
		// Without this, every installed/running arrow renders as `'absent'`
		// forever, since no transition frame is coming to correct it.
		let pendingInitialStates: ReturnType<typeof toInitialRuntimeUpdates> | null = null;

		// The ENTITY stream: complete catalog DTOs, authoritative, seeds and
		// prunes. `/v0/arrow` upgrades to a WS on the same path (design §1).
		disposeArrowStream = subscribeArrowStream({
			connectionId,
			seed: () =>
				apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true').then((items) => {
					pendingInitialStates = toInitialRuntimeUpdates(items);
					return toArrowCatalogRecords(items, connectionId);
				}),
			onChange: () => {
				getArrowsFor(connectionId).then((records) => {
					useArrowStore.getState().setCatalog(records);
					// entity-stream chains applySeed's own onChange strictly before
					// any live frame's onChange on the same serial `applyChain`, so
					// THIS onChange call (the one right after a seed) is guaranteed
					// to be the seed's — never a live frame's — meaning this runs
					// exactly once per seed/reseed and can never reapply a stale
					// snapshot over a fresher live delta on a later, unrelated
					// onChange. `seedInitialState` itself additionally refuses to
					// overwrite an overlay a live `/v0/runtime` frame already set.
					if (pendingInitialStates) {
						const initial = pendingInitialStates;
						pendingInitialStates = null;
						initial.forEach((update) => useArrowStore.getState().seedInitialState(update));
					}
				});
			},
		});

		// The OVERLAY stream: patches state/active_run/last_return onto existing
		// entries only. Never creates or prunes — enforced by the store itself
		// (applyRuntimeUpdate no-ops on an unknown namespace).
		disposeRuntimeStream = wsManager.subscribe(RUNTIME_ENDPOINT, (data) => {
			// The reconnect sentinel is not a DTO. Unlike the entity stream, the
			// overlay never reseeds on it: the next live frame simply patches
			// state again, and there is nothing here to prune or resurrect.
			if (isReconnectSentinel(data)) return;
			useArrowStore.getState().applyRuntimeUpdate(toRuntimeUpdate(data as RuntimeUpdateDTO));
		});
	}

	await listen<{ status: ConnectionStatus }>('core://status', async (e) => {
		useStatusStore.getState().setStatus(e.payload.status);
		if (e.payload.status === 'starting') {
			// Dispose both subscriptions and reset the projection — but do NOT
			// wipe the cache. The old partition stays on disk so switching back
			// paints instantly (design decision #3).
			stopStreams();
			useArrowStore.getState().reset();
		}
		if (e.payload.status === 'ready') {
			// Self-sufficient: query the connection the Rust side actually has
			// active right now, rather than trusting a `connection://changed`
			// event that a switch does not currently emit (see the module doc
			// comment above).
			const { active_id } = await invoke<GetConnectionsResponse>('get_connections');
			startStreams(active_id);
		}
	});

	await listen<{ connections: ConnectionConfig[]; active_id: string }>('connection://changed', () => {
		// Intentionally a no-op here — see the module doc comment above.
	});
}
