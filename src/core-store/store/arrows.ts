import { create } from 'zustand';

import type { ArrowListItem } from '@/domain/arrow';

interface RuntimeUpdatePayload {
	namespace: string;
	state: ArrowListItem['state'];
	active_run: ArrowListItem['active_run'];
	last_outcome: ArrowListItem['last_outcome'];
}

interface ArrowStore {
	arrows: Map<string, ArrowListItem>;
	upsertArrow: (item: ArrowListItem) => void;
	removeArrow: (namespace: string) => void;
	hydrateArrows: (items: ArrowListItem[]) => void;
	applyRuntimeUpdate: (payload: RuntimeUpdatePayload) => void;
	resetArrows: () => void;
}

export const useArrowStore = create<ArrowStore>((set) => ({
	arrows: new Map(),

	upsertArrow: (item) =>
		set((s) => {
			const next = new Map(s.arrows);
			next.set(item.namespace, item);
			return { arrows: next };
		}),

	removeArrow: (namespace) =>
		set((s) => {
			const next = new Map(s.arrows);
			next.delete(namespace);
			return { arrows: next };
		}),

	hydrateArrows: (items) =>
		set((s) => {
			const next = new Map(s.arrows);
			for (const item of items) {
				next.set(item.namespace, item);
			}
			return { arrows: next };
		}),

	applyRuntimeUpdate: (payload) =>
		set((s) => {
			const existing = s.arrows.get(payload.namespace);
			if (!existing) return s;
			const next = new Map(s.arrows);
			next.set(payload.namespace, {
				...existing,
				state: payload.state,
				active_run: payload.active_run,
				last_outcome: payload.last_outcome ?? existing.last_outcome,
			});
			return { arrows: next };
		}),

	resetArrows: () => set({ arrows: new Map() }),
}));
