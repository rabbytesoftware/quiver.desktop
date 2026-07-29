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
	 * the first live transition). Unlike `applyRuntimeUpdate`, this never
	 * overwrites an overlay already established by a live `/v0/runtime`
	 * frame — but a NEWER seed IS allowed to replace an OLDER seed's own
	 * value (see the `liveNamespaces` comment below for why that distinction
	 * matters). Still requires the namespace to already be in the catalog,
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

// Resolves what an update means against what's already known, BEFORE it is
// stored — not just what's applied to the entry. Storing the raw `update` in
// `runtime` (rather than this resolved value) was fix round 1's bug: the
// NEXT setCatalog re-derives every entry from `runtime` via toEntry, so a
// raw update with `last_return: null` would re-wipe the very outcome
// applyRuntimeUpdate had just preserved on the entry, the moment any catalog
// frame (or reseed) arrived afterward — which `listeners/index.ts` triggers
// on every entity-stream `onChange`, i.e. constantly.
function resolveOverlay(existing: ArrowEntry, update: RuntimeUpdate): RuntimeUpdate {
	return {
		namespace: update.namespace,
		state: update.state,
		active_run: update.active_run,
		last_return: update.last_return ?? existing.last_return,
	};
}

function patchOverlay(existing: ArrowEntry, overlay: RuntimeUpdate): ArrowEntry {
	return {
		...existing,
		state: overlay.state,
		active_run: overlay.active_run,
		last_return: overlay.last_return,
	};
}

export const useArrowStore = create<ArrowStore>((set, get) => {
	// In-memory only — runtime state is never persisted (design §5.4). Kept
	// outside the reactive `arrows` map so setCatalog can consult a namespace's
	// last-known runtime state without it having been part of the catalog
	// payload that triggered the reseed. Values are always the RESOLVED
	// overlay (see resolveOverlay), never a raw, possibly-partial update.
	let runtime = new Map<string, RuntimeUpdate>();
	// Which namespaces' CURRENT `runtime` entry came from a live `/v0/runtime`
	// frame (applyRuntimeUpdate) rather than a catalog seed (seedInitialState).
	// The distinction matters for a narrow but real race: a reconnect sentinel
	// arriving while the FIRST seed's GET is still in flight can make that
	// seed's own onChange apply its (now superseded) state before the NEWER
	// reseed's onChange ever runs — entity-stream's onChange fires even for a
	// bailed/superseded seed. Without this set, seedInitialState's "don't
	// downgrade an existing overlay" guard would see the older seed's value
	// sitting in `runtime` and refuse the newer seed's fresher one, so stale
	// would win permanently. A live delta must NEVER be downgraded by any
	// seed; an older SEED's value may always be replaced by a newer one,
	// since entity-stream's serial chain guarantees seed onChanges fire in
	// generation order even when an earlier one was itself superseded.
	let liveNamespaces = new Set<string>();

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
				if (!stillPresent.has(namespace)) {
					runtime.delete(namespace);
					liveNamespaces.delete(namespace);
				}
			}
			set({ arrows: next });
		},

		applyRuntimeUpdate: (update) => {
			const existing = get().arrows.get(update.namespace);
			// Unknown namespace: the catalog stream hasn't seeded it (or never
			// will). Dropping the frame here is correct, not a bug — see the
			// interface doc comment above.
			if (!existing) return;
			const resolved = resolveOverlay(existing, update);
			runtime = new Map(runtime).set(update.namespace, resolved);
			liveNamespaces = new Set(liveNamespaces).add(update.namespace);
			const next = new Map(get().arrows);
			next.set(update.namespace, patchOverlay(existing, resolved));
			set({ arrows: next });
		},

		seedInitialState: (update) => {
			const existing = get().arrows.get(update.namespace);
			if (!existing) return;
			// Never downgrade an overlay a LIVE `/v0/runtime` frame established.
			// An older SEED's own value, in contrast, may always be replaced by a
			// newer one — see the `liveNamespaces` comment above.
			if (liveNamespaces.has(update.namespace)) return;
			const resolved = resolveOverlay(existing, update);
			runtime = new Map(runtime).set(update.namespace, resolved);
			const next = new Map(get().arrows);
			next.set(update.namespace, patchOverlay(existing, resolved));
			set({ arrows: next });
		},

		reset: () => {
			runtime = new Map();
			liveNamespaces = new Set();
			set({ arrows: new Map() });
		},
	};
});
