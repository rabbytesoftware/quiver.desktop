import { create } from 'zustand';

/** Apps kept alive at once; the least recently used one beyond this goes. */
export const MAX_OPEN_APPS = 4;

/** Marks `ns` most recently used and evicts down to `cap`, sparing `visible`. */
export function admit(
	order: readonly string[],
	ns: string,
	visible: string | null,
	cap: number = MAX_OPEN_APPS
): string[] {
	const next = [...order.filter((n) => n !== ns), ns];
	while (next.length > cap) {
		const victim = next.findIndex((n) => n !== visible && n !== ns);
		if (victim === -1) break;
		next.splice(victim, 1);
	}
	return next;
}

interface OpenAppsState {
	/** Open apps, least recently used first. Decides eviction only. */
	order: string[];
	/**
	 * Open apps in the order they were opened, which is the order the frames
	 * render in. It never follows `order`: moving an iframe in the DOM reloads it.
	 */
	frames: string[];
	visible: string | null;
	/** Bumped to remount an app's frame. */
	reloads: Record<string, number>;
	show(ns: string): void;
	hide(): void;
	reload(ns: string): void;
	prune(live: ReadonlySet<string>): void;
}

export const useOpenApps = create<OpenAppsState>((set) => ({
	order: [],
	frames: [],
	visible: null,
	reloads: {},
	show: (ns) =>
		set((s) => {
			const order = admit(s.order, ns, ns);
			const frames = [...s.frames.filter((n) => order.includes(n)), ...(s.frames.includes(ns) ? [] : [ns])];
			return { visible: ns, order, frames };
		}),
	hide: () => set({ visible: null }),
	reload: (ns) => set((s) => ({ reloads: { ...s.reloads, [ns]: (s.reloads[ns] ?? 0) + 1 } })),
	prune: (live) =>
		set((s) => {
			const order = s.order.filter((n) => live.has(n));
			if (order.length === s.order.length) return s;
			const frames = s.frames.filter((n) => live.has(n));
			const reloads = Object.fromEntries(Object.entries(s.reloads).filter(([n]) => live.has(n)));
			return { order, frames, reloads };
		}),
}));
