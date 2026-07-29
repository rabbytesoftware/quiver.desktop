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
	 * catalog stream's job alone.
	 */
	applyRuntimeUpdate: (update: RuntimeUpdate) => void;
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
			for (const record of records) {
				next.set(record.namespace, toEntry(record, runtime.get(record.namespace)));
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
			next.set(update.namespace, {
				...existing,
				state: update.state,
				active_run: update.active_run,
				last_return: update.last_return,
			});
			set({ arrows: next });
		},

		reset: () => {
			runtime = new Map();
			set({ arrows: new Map() });
		},
	};
});
