import { create } from 'zustand';

import type { ArrowEntry, ArrowState, RuntimeUpdate } from '@/domain/arrow';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

const NEUTRAL_STATE: ArrowState = 'absent';

// The runtime endpoint broadcasts every arrow the daemon manages, not just
// this catalog's own (user-installed) rows -- a dependency pulled in by an
// install transitions through its own states on the same stream, and since
// a pure dependency never appears in `records` (the catalog seed is always
// filtered to user-installed arrows), its buffered update would otherwise
// sit in `pendingRuntime` for the rest of the session. This cap keeps that
// bounded: once exceeded, the oldest buffered entry (a Map iterates in
// insertion order) is dropped to make room for the newest.
const MAX_PENDING_RUNTIME = 256;

export type CatalogStatus = 'loading' | 'ready' | 'error';

interface ArrowStore {
	arrows: Map<string, ArrowEntry>;
	catalog: CatalogStatus;
	setCatalog: (records: ArrowCatalogRecord[]) => void;
	setCatalogError: () => void;
	applyRuntimeUpdate: (update: RuntimeUpdate) => void;
	seedInitialState: (update: RuntimeUpdate) => void;
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
		last_used_at: record.last_used_at,
	};
}

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
	let runtime = new Map<string, RuntimeUpdate>();
	let liveNamespaces = new Set<string>();
	// A runtime frame for a namespace whose catalog entry hasn't landed yet
	// (announceSelf's own POST races its two independent WS streams -- the
	// catalog upsert and the runtime transition it triggers arrive in either
	// order). Kept separate from `runtime` so the pruning loop below -- which
	// can't tell "removed" apart from "not seeded yet" -- never discards it
	// before setCatalog gets a chance to consume it. Drained (never pruned)
	// on first sight of the namespace in setCatalog, or wholesale on reset.
	let pendingRuntime = new Map<string, RuntimeUpdate>();

	return {
		arrows: new Map(),
		catalog: 'loading',

		setCatalogError: () => set({ catalog: 'error' }),

		setCatalog: (records) => {
			const previous = get().arrows;
			const next = new Map<string, ArrowEntry>();
			const stillPresent = new Set<string>();
			for (const record of records) {
				stillPresent.add(record.namespace);
				let overlay = runtime.get(record.namespace);
				const pending = pendingRuntime.get(record.namespace);
				if (!overlay && !previous.has(record.namespace) && pending) {
					overlay = pending;
					runtime = new Map(runtime).set(record.namespace, pending);
					liveNamespaces = new Set(liveNamespaces).add(record.namespace);
					const remainingPending = new Map(pendingRuntime);
					remainingPending.delete(record.namespace);
					pendingRuntime = remainingPending;
				}
				next.set(record.namespace, toEntry(record, overlay));
			}
			for (const namespace of runtime.keys()) {
				if (!stillPresent.has(namespace)) {
					runtime.delete(namespace);
					liveNamespaces.delete(namespace);
				}
			}
			set({ arrows: next, catalog: 'ready' });
		},

		applyRuntimeUpdate: (update) => {
			const existing = get().arrows.get(update.namespace);
			if (!existing) {
				const next = new Map(pendingRuntime);
				next.delete(update.namespace);
				if (next.size >= MAX_PENDING_RUNTIME) {
					const oldest = next.keys().next().value;
					if (oldest !== undefined) next.delete(oldest);
				}
				next.set(update.namespace, update);
				pendingRuntime = next;
				return;
			}
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
			pendingRuntime = new Map();
			set({ arrows: new Map(), catalog: 'loading' });
		},
	};
});
