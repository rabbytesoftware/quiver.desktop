import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { RuntimeUpdate } from '@/domain/arrow';
import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch, coreIsReachable } from '@/lib/transport/api';
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
 *
 * The event is also not TRUSTED to arrive: registration races Rust's own
 * `ready`, which Tauri drops if it lands first. `adoptRunningCore` below closes
 * that hole by asking the daemon directly once registration is done.
 */
export async function setupListeners(): Promise<void> {
	// Started here, awaited only where it matters — NOT before `listen()` below.
	//
	// The wipe must still be complete before any seed touches the cache (design
	// decision #4), and it is: `beginStreams` awaits it, and `beginStreams` is
	// the only route to `startStreams` and therefore to the cache. But AWAITING
	// it here would put a full IndexedDB open plus `clear()` between app start
	// and the only thing that can catch `core://status` — and an event with no
	// registered listener is dropped, not buffered (see `adoptRunningCore`).
	// Registration must be the first thing that happens.
	//
	// It cannot reject: `maybeWipeOnVersionChange` catches every failure
	// internally, because a cache must never break the app. So leaving this
	// promise un-awaited for the length of one `listen()` cannot produce an
	// unhandled rejection.
	const wipeDone = maybeWipeOnVersionChange();

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
		// Fix round 4, R2: this used to be a plain mutable slot that `onChange`
		// partitioned in place (apply what's visible, retain the rest for a
		// later `onChange`). That retained a batch FOREVER if its own seed
		// bailed (nothing visible, nothing applied — entity-stream still fires
		// onChange for a bailed seed) and the correcting reseed then also
		// failed (a thrown seed() skips onChange entirely, so nothing ever
		// clears it) — an unrelated namespace installed for the first time much
		// later, sharing a name with something in the abandoned batch, would
		// then be mispainted with that stale state. Fixed with IDENTITY: each
		// seed() call produces a BRAND NEW array (never mutated in place), and
		// `onChange` captures a reference to it SYNCHRONOUSLY, the instant
		// entity-stream invokes `onChange` — which, by entity-stream's own
		// chaining, is always immediately after THIS seed's own processing
		// finished, bailed, or was about to throw. That capture point is the
		// one and only `onChange` invocation this specific batch may ever be
		// judged by:
		//  - if `pendingInitialStates` no longer points at the captured
		//    reference by the time this `onChange`'s own read resolves, a
		//    NEWER seed has already produced its own batch — this one is stale
		//    and is discarded outright, not partially applied (reapplying an
		//    older snapshot over whatever the newer batch's own onChange has
		//    since done would silently regress it);
		//  - otherwise, whatever's visible in this read is applied, and the
		//    slot is cleared — ALWAYS, win or lose, once per batch. A batch
		//    whose own seed bailed sees nothing, applies nothing, and is
		//    discarded right there; it can never resurface for a later,
		//    unrelated onChange to misapply.
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
			onChange: () => {
				const myBatch = pendingInitialStates;
				return getArrowsFor(connectionId).then((records) => {
					// A `starting` (and possibly a whole new `ready`) landed while
					// this read was in flight for a connection that is no longer
					// current — drop it rather than writing a defunct connection's
					// rows back into the store after `reset()` already ran.
					if (generation !== streamGeneration) return;
					useArrowStore.getState().setCatalog(records);
					// See the long comment above `pendingInitialStates`: a batch
					// that's no longer the current one belongs to a superseded
					// seed and must not be applied at all, partially or otherwise.
					if (pendingInitialStates !== myBatch) return;
					if (myBatch.length === 0) return;
					const visible = new Set(records.map((r) => r.namespace));
					myBatch
						.filter((update) => visible.has(update.namespace))
						.forEach((update) => useArrowStore.getState().seedInitialState(update));
					// One shot, always — whether this attempt saw everything,
					// something, or nothing.
					pendingInitialStates = [];
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

	// The one route to `startStreams`, and therefore the one place that has to
	// hold both invariants: the cache wipe is complete before anything seeds
	// (design decision #4), and a `starting` that landed while this was in
	// flight wins.
	async function beginStreams(myGeneration: number): Promise<void> {
		await wipeDone;
		// Self-sufficient: query the connection the Rust side actually has
		// active right now, rather than trusting a `connection://changed`
		// event that a switch does not currently emit (see the module doc
		// comment above).
		const { active_id } = await invoke<GetConnectionsResponse>('get_connections');
		// A `starting` landed while the calls above were in flight — this start
		// has been superseded (see the `generation` doc comment above). Its own
		// streams were never started, so there is nothing to dispose; just drop
		// the stale result.
		if (generation !== myGeneration) return;
		startStreams(active_id);
	}

	/**
	 * Catch a `ready` that was emitted before the listener above existed.
	 *
	 * `emit_core_status` is fire-and-forget and Tauri DROPS an event with no
	 * registered listener — no buffering, no replay, and there is no command to
	 * ask for the status either. Meanwhile Rust emits `ready` on its FIRST
	 * successful `/v0/health`, with no initial delay, so against a daemon that
	 * is already up — a previous run's orphaned sidecar answering on a warm
	 * socket, or a developer's own `quiver daemon` — that emit happens
	 * milliseconds after `setup()` and can beat this webview's registration.
	 *
	 * Nothing downstream recovers from that on its own: `switch_connection`
	 * emits no `connection://changed`, and `useStatusStore` defaults to
	 * 'starting', so a missed `ready` is indistinguishable from a daemon that
	 * never came up. The app sits empty forever.
	 *
	 * So don't depend on catching a transient event — ask. `coreIsReachable()`
	 * is one unenveloped GET at `/v0/health`, the same question
	 * `sidecar::wait_for_ready` asks, and the only one this module actually
	 * cares about: can I read data right now? A cached last-emitted status
	 * would be a second source of truth that can go stale, and its own default
	 * would be the same ambiguous 'starting'; a daemon that answers cannot lie.
	 */
	async function adoptRunningCore(): Promise<void> {
		const myGeneration = generation;
		if (!(await coreIsReachable())) return;
		// The event beat the probe after all — either a `starting` landed (the
		// channel is alive and a `ready` for the new connection is coming, so
		// adopting now would start streams mid-restart), or a `ready` already
		// started them (restarting would tear down a healthy pair for a needless
		// WS reconnect and a duplicate catalog GET).
		if (generation !== myGeneration || disposeArrowStream) return;
		useStatusStore.getState().setStatus('ready');
		await beginStreams(myGeneration);
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
			await beginStreams(generation);
		}
	});

	await adoptRunningCore();
}
