import { create } from 'zustand';

import type { ArrowEntry, RuntimeUpdate } from '@/domain/arrow';

interface ArrowStore {
	arrows: Map<string, ArrowEntry>;
	upsertArrow: (item: ArrowEntry) => void;
	removeArrow: (namespace: string) => void;
	applyRuntimeUpdate: (payload: RuntimeUpdate) => void;
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

	applyRuntimeUpdate: (payload) =>
		set((s) => {
			const existing = s.arrows.get(payload.namespace);
			if (!existing) return s;
			const next = new Map(s.arrows);
			next.set(payload.namespace, {
				...existing,
				state: payload.state,
				active_run: payload.active_run,
				last_return: payload.last_return ?? existing.last_return,
			});
			return { arrows: next };
		}),

	resetArrows: () => set({ arrows: new Map() }),
}));
