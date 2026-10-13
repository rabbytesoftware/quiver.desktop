import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export const BUILD_INDICATOR_STORAGE_KEY = 'quiver.buildIndicator';

interface BuildIndicatorState {
	/** `null` means the person has not chosen, so the build's own channel decides. */
	shown: boolean | null;
	setShown: (shown: boolean) => void;
	reset: () => void;
}

export function normaliseShown(value: unknown): boolean | null {
	return typeof value === 'boolean' ? value : null;
}

export const useBuildIndicatorStore = create<BuildIndicatorState>()(
	persist(
		(set) => ({
			shown: null,
			setShown: (shown) => set({ shown }),
			reset: () => set({ shown: null }),
		}),
		{
			name: BUILD_INDICATOR_STORAGE_KEY,
			partialize: (s) => ({ shown: s.shown }),
			merge: (persisted, current) => ({
				...current,
				shown: normaliseShown((persisted as { shown?: unknown } | null)?.shown),
			}),
		}
	)
);
