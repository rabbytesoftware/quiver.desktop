import { create } from 'zustand';

import type { ArrowEntry, ArrowState, RuntimeUpdate } from '@/domain/arrow';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

const NEUTRAL_STATE: ArrowState = 'absent';

interface ArrowStore {
	arrows: Map<string, ArrowEntry>;
	setCatalog: (records: ArrowCatalogRecord[]) => void;
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

	return {
		arrows: new Map(),

		setCatalog: (records) => {
			const next = new Map<string, ArrowEntry>();
			const stillPresent = new Set<string>();
			for (const record of records) {
				stillPresent.add(record.namespace);
				next.set(record.namespace, toEntry(record, runtime.get(record.namespace)));
			}
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
