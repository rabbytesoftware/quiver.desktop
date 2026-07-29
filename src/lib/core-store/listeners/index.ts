import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { RuntimeUpdate } from '@/domain/arrow';
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
 * `core://status` is the only Tauri event this module listens for.
 * `connection://changed` still exists (add/remove/rename emit it) but is
 * consumed by `@/lib/connection/listeners.ts` for the connections UI, not
 * here: `switch_connection` does not emit it at all (a known Rust-side gap —
 * `manager.rs` awaits `guard.start(app)`, which emits `starting` then
 * `ready` before the command returns, so even an appended emit would land
 * after the `ready` that already restarted the streams). Instead, `ready`
 * below calls `get_connections` directly, which makes it self-sufficient
 * and immune to that ordering/emission gap. `arrow://event` and
 * `runtime://update` no longer exist at all — the arrow catalog and the
 * runtime overlay now ride the transport layer (`apiFetch` + `wsManager`).
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
	// Bumped on every `starting`. Tauri does NOT serialize async event
	// handlers — a `starting` can land while an earlier `ready`'s own async
	// work (its `get_connections` call, or a stream's own cache read below)
	// is still in flight. Without checking this, that in-flight work would
	// resolve AFTER `stopStreams()`/`reset()` already ran and act anyway —
	// resurrecting state for a connection that's no longer active. Same
	// pattern entity-stream.ts already uses for a superseded reseed.
	let generation = 0;

	function stopStreams(): void {
		disposeArrowStream?.();
		disposeArrowStream = null;
		disposeRuntimeStream?.();
		disposeRuntimeStream = null;
	}

	function startStreams(connectionId: string): void {
		// Defensive, not required by any currently-reachable path: every
		// CoreStatus::Ready is preceded by CoreStatus::Starting within the same
		// Rust-side start() call, and manager.rs holds the `active` RwLock write
		// guard across `guard.start(app).await`, serializing switches — so this
		// function should never run twice without an intervening stopStreams()
		// today. Calling it here anyway makes that structurally guaranteed
		// rather than incidental, in case that Rust invariant ever changes. Free
		// insurance; no test guards this specifically (fix round 3, R3) — the
		// path is unreachable under the current Rust contract, and a test for
		// an unreachable path would only assert the mock harness we built, not
		// real behaviour.
		stopStreams();

		// This stream's own identity, fixed at the moment it starts. `onChange`
		// below (an async read) checks this AFTER its await resolves — if a
		// `starting` landed in the meantime, `generation` has moved on and the
		// stale result is dropped rather than resurrecting this now-defunct
		// connection's rows into the (already reset) store (fix round 3, R1).
		const streamGeneration = generation;

		// Neither `/v0/arrow` nor `/v0/runtime` push anything on connect (both
		// are transition-only sockets, verified directly against stable-26.5.1)
		// — so the seed GET's own `versions[].state` is the ONLY source for a
		// correct initial paint. Without this, every installed/running arrow
		// renders as `'absent'` forever, since no transition frame is coming to
		// correct it.
		//
		// A flat, mutable batch — NOT a single "pending or not" slot keyed to
		// exactly one seed. `onChange` below only ever removes the entries whose
		// namespace it can actually SEE in what it just read from the cache,
		// leaving the rest for a later `onChange` call to pick up. This is what
		// makes it safe against a reseed's GET resolving before an EARLIER
		// onChange's own cache read completes: an earlier, stale-snapshot
		// onChange can only ever consume states for namespaces it can observe,
		// so it can never silently discard a namespace it hasn't seen yet — a
		// later onChange, once its own read does see it, still applies it (fix
		// round 3, R2). A single overwritten-and-cleared slot could not
		// guarantee that: the reviewer's probe showed a namespace present only
		// in the newer catalog having its seeded state discarded and rendering
		// `'absent'` forever.
		let pendingInitialStates: RuntimeUpdate[] = [];

		// The ENTITY stream: complete catalog DTOs, authoritative, seeds and
		// prunes. `/v0/arrow` upgrades to a WS on the same path (design §1).
		disposeArrowStream = subscribeArrowStream({
			connectionId,
			seed: () =>
				apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true').then((items) => {
					pendingInitialStates = toInitialRuntimeUpdates(items);
					return toArrowCatalogRecords(items, connectionId);
				}),
			onChange: () =>
				getArrowsFor(connectionId).then((records) => {
					// A `starting` (and possibly a whole new `ready`) landed while
					// this read was in flight for a connection that is no longer
					// current — drop it rather than writing a defunct connection's
					// rows back into the store after `reset()` already ran.
					if (generation !== streamGeneration) return;
					useArrowStore.getState().setCatalog(records);
					if (pendingInitialStates.length === 0) return;
					const visible = new Set(records.map((r) => r.namespace));
					const applicable: RuntimeUpdate[] = [];
					const stillPending: RuntimeUpdate[] = [];
					for (const update of pendingInitialStates) {
						(visible.has(update.namespace) ? applicable : stillPending).push(update);
					}
					pendingInitialStates = stillPending;
					applicable.forEach((update) => useArrowStore.getState().seedInitialState(update));
				}),
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
			generation++;
			stopStreams();
			useArrowStore.getState().reset();
		}
		if (e.payload.status === 'ready') {
			const myGeneration = generation;
			// Self-sufficient: query the connection the Rust side actually has
			// active right now, rather than trusting a `connection://changed`
			// event that a switch does not currently emit (see the module doc
			// comment above).
			const { active_id } = await invoke<GetConnectionsResponse>('get_connections');
			// A `starting` landed while the call above was in flight — this
			// `ready` has been superseded (see the `generation` doc comment
			// above). Its own streams were never started, so there is nothing
			// to dispose; just drop the stale result.
			if (generation !== myGeneration) return;
			startStreams(active_id);
		}
	});
}
