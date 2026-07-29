import { create } from 'zustand';

import type { ArrowEntry, ArrowState, RuntimeUpdate } from '@/domain/arrow';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

// A cold start paints from cache. If runtime state came from disk it would
// claim "running" about a process the daemon may have killed hours ago — so
// every freshly-projected entry starts neutral until the runtime socket (or a
// live catalog frame carrying an update) says otherwise. See design §5.4.
const NEUTRAL_STATE: ArrowState = 'absent';

interface ArrowStore {
	/** Catalog rows composed with any in-memory runtime overlay. */
	arrows: Map<string, ArrowEntry>;
	/**
	 * Wholesale replace of the catalog (an initial seed or a reconnect reseed).
	 * Preserves the runtime overlay already applied for any namespace still
	 * present in the new set — a reconnect must not blank the UI.
	 */
	setCatalog: (records: ArrowCatalogRecord[]) => void;
	/**
	 * Patches state/active_run/last_return onto an EXISTING entry only. The
	 * runtime overlay stream must never create or prune a row — that is the
	 * catalog stream's job alone. `active_run` is always overwritten (null is
	 * a meaningful "nothing running right now"); `last_return` falls back to
	 * whatever was already known when the frame omits it, so a state-only
	 * transition frame does not wipe the last known outcome from the UI.
	 */
	applyRuntimeUpdate: (update: RuntimeUpdate) => void;
	/**
	 * Seeds a namespace's INITIAL runtime state from the catalog GET's own
	 * `versions[].state` (see `toInitialRuntimeUpdates` — neither stream
	 * pushes anything on connect, so this is the only source of truth until
	 * the first live transition). Unlike `applyRuntimeUpdate`, this is a
	 * "set only if nothing is known yet" — it never overwrites an overlay
	 * already established by a live `/v0/runtime` frame, so a reseed (whose
	 * GET may lag behind an in-flight transition) can never stomp fresher
	 * live data. Still requires the namespace to already be in the catalog,
	 * same invariant as `applyRuntimeUpdate`.
	 */
	seedInitialState: (update: RuntimeUpdate) => void;
	/** Clears the projection (used on a connection switch's `starting` phase).
	 *  Does not touch the on-disk cache — the caller decides that. */
	reset: () => void;
}

function toEntry(record: ArrowCatalogRecord, overlay: RuntimeUpdate | undefined): ArrowEntry {
	return {
		namespace: record.namespace,
		name: record.name,
		description: record.description,
		tags: record.tags,
		icon: record.icon,
		banner: record.banner,
		version: record.version,
		state: overlay?.state ?? NEUTRAL_STATE,
		active_run: overlay?.active_run ?? null,
		last_return: overlay?.last_return ?? null,
	};
}

// Shared by applyRuntimeUpdate and seedInitialState: active_run is always
// overwritten (null is meaningful — "nothing running right now"); last_return
// falls back to what's already known when the update omits it, so a
// state-only transition frame never wipes the last outcome from the UI.
function patchOverlay(existing: ArrowEntry, update: RuntimeUpdate): ArrowEntry {
	return {
		...existing,
		state: update.state,
		active_run: update.active_run,
		last_return: update.last_return ?? existing.last_return,
	};
}

export const useArrowStore = create<ArrowStore>((set, get) => {
	// In-memory only — runtime state is never persisted (design §5.4). Kept
	// outside the reactive `arrows` map so setCatalog can consult a namespace's
	// last-known runtime state without it having been part of the catalog
	// payload that triggered the reseed.
	let runtime = new Map<string, RuntimeUpdate>();

	return {
		arrows: new Map(),

		setCatalog: (records) => {
			const next = new Map<string, ArrowEntry>();
			const stillPresent = new Set<string>();
			for (const record of records) {
				stillPresent.add(record.namespace);
				next.set(record.namespace, toEntry(record, runtime.get(record.namespace)));
			}
			// Prune overlay entries for namespaces the fresh catalog no longer
			// lists — otherwise `runtime` grows unboundedly across reconnects, and
			// a namespace that's uninstalled then later reinstalled would inherit
			// its previous life's stale overlay instead of a fresh seed.
			for (const namespace of runtime.keys()) {
				if (!stillPresent.has(namespace)) runtime.delete(namespace);
			}
			set({ arrows: next });
		},

		applyRuntimeUpdate: (update) => {
			const existing = get().arrows.get(update.namespace);
			// Unknown namespace: the catalog stream hasn't seeded it (or never
			// will). Dropping the frame here is correct, not a bug — see the
			// interface doc comment above.
			if (!existing) return;
			runtime = new Map(runtime).set(update.namespace, update);
			const next = new Map(get().arrows);
			next.set(update.namespace, patchOverlay(existing, update));
			set({ arrows: next });
		},

		seedInitialState: (update) => {
			const existing = get().arrows.get(update.namespace);
			if (!existing) return;
			// Already has live overlay data (from a prior seed or a `/v0/runtime`
			// frame) — never downgrade it with a possibly-stale GET snapshot.
			if (runtime.has(update.namespace)) return;
			runtime = new Map(runtime).set(update.namespace, update);
			const next = new Map(get().arrows);
			next.set(update.namespace, patchOverlay(existing, update));
			set({ arrows: next });
		},

		reset: () => {
			runtime = new Map();
			set({ arrows: new Map() });
		},
	};
});
