import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import { LOCAL_CONNECTION_ID } from '@/domain/connection';
import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch } from '@/lib/transport/api';
import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import type { ArrowListResponseItemDTO } from '../dtos/v0/arrow';
import { toArrowCatalogRecords } from '../dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

const RUNTIME_ENDPOINT = '/v0/runtime';

interface ConnectionChangedPayload {
	connections: ConnectionConfig[];
	active_id: string;
}

/**
 * Only two Tauri events survive the data-layer rewrite: `core://status` and
 * `connection://changed`. Both are native lifecycle facts the webview cannot
 * observe on its own (sidecar readiness, and which backend the Rust side has
 * switched to). Everything else — the arrow catalog and the runtime overlay —
 * now rides the transport layer (`apiFetch` + `wsManager`) instead of the
 * bespoke `arrow://event` / `runtime://update` events, which no longer exist.
 */
export async function setupListeners(): Promise<void> {
	// Must run before any seed touches the cache (design decision #4).
	await maybeWipeOnVersionChange();

	// Local to this call, not module-level: setupListeners runs exactly once
	// per app lifetime in production, so a closure gives every listener a
	// consistent view of "which connection, which live subscriptions" without
	// leaking state across repeated calls (e.g. in tests).
	let activeConnectionId = LOCAL_CONNECTION_ID;
	let disposeArrowStream: (() => void) | null = null;
	let disposeRuntimeStream: (() => void) | null = null;

	function stopStreams(): void {
		disposeArrowStream?.();
		disposeArrowStream = null;
		disposeRuntimeStream?.();
		disposeRuntimeStream = null;
	}

	function startStreams(connectionId: string): void {
		// The ENTITY stream: complete catalog DTOs, authoritative, seeds and
		// prunes. `/v0/arrow` upgrades to a WS on the same path (design §1).
		disposeArrowStream = subscribeArrowStream({
			connectionId,
			seed: () =>
				apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true').then((items) =>
					toArrowCatalogRecords(items, connectionId)
				),
			onChange: () => {
				getArrowsFor(connectionId).then((records) => useArrowStore.getState().setCatalog(records));
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

	await listen<{ status: ConnectionStatus }>('core://status', (e) => {
		useStatusStore.getState().setStatus(e.payload.status);
		if (e.payload.status === 'starting') {
			// Dispose both subscriptions and reset the projection — but do NOT
			// wipe the cache. The old partition stays on disk so switching back
			// paints instantly (design decision #3).
			stopStreams();
			useArrowStore.getState().reset();
		}
		if (e.payload.status === 'ready') {
			startStreams(activeConnectionId);
		}
	});

	await listen<ConnectionChangedPayload>('connection://changed', (e) => {
		// Recorded for the NEXT `ready` — a switch always passes through
		// `starting` first, which is what actually tears down the old streams.
		activeConnectionId = e.payload.active_id;
	});
}
