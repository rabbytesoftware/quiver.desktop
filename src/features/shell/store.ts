import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { SidebarSide } from './geometry';

/**
 * The rail's floor, in CSS px. DERIVED, not chosen.
 *
 * The active nav segment is capped at 54% of the rail and the two collapsed
 * segments each have a `--row` (34px) floor, so all three fit only while
 * `0.54W + 2·34 ≤ W` — that is, `W ≥ 148`, rounded up to 160. Below it the
 * segments stop shrinking and overflow the rail instead.
 *
 * The retired `store/ui.ts` used 120, which came from a 208px rail with the
 * search field inside it. That layout is gone; carrying its floor forward would
 * reopen the overflow at every width between 120 and 148.
 */
export const SIDEBAR_MIN = 160;

/** The rail's ceiling, in CSS px. Past it the rail reads as a second column. */
export const SIDEBAR_MAX = 320;

/** `--rail`'s value in index.css. The two must move together. */
export const SIDEBAR_DEFAULT = 246;

export const SHELL_STORAGE_KEY = 'quiver.shell';

export interface ShellState {
	/** Which edge the rail is docked to. Persisted. */
	sidebarSide: SidebarSide;
	/** The rail's width in CSS px, always within [SIDEBAR_MIN, SIDEBAR_MAX]. Persisted. */
	sidebarWidth: number;
	setSidebarSide: (side: SidebarSide) => void;
	setSidebarWidth: (width: number) => void;
}

/**
 * A stored side is a string on disk, and `SidebarSide` does not reach it. A
 * hand-edited `quiver.shell` — or a build that once wrote something else — would
 * otherwise hand the grid a column order it has no rule for, and the rail would
 * render on top of the content column with no setting on screen to undo it.
 *
 * Anything that is not `'right'` is `'left'`, so the default survives every
 * shape of garbage without a second branch. Same reasoning as
 * `normalisePreference` in `@/lib/i18n/store`.
 */
export function normaliseSide(value: unknown): SidebarSide {
	return value === 'right' ? 'right' : 'left';
}

/**
 * A width that is not a finite number reaches `calc()` as garbage and voids the
 * whole declaration — the rail renders at zero and takes the resize handle with
 * it. A finite one that is merely out of range is just as fatal at the top end:
 * 99999px pushes the content column off the window.
 *
 * Called on both paths on purpose. Rehydration is where `"wide"` and `null`
 * arrive; the setter is where a drag past either end does.
 */
export function normaliseWidth(value: unknown): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) return SIDEBAR_DEFAULT;
	return Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, value));
}

export const useShellStore = create<ShellState>()(
	persist(
		(set) => ({
			sidebarSide: 'left',
			sidebarWidth: SIDEBAR_DEFAULT,

			setSidebarSide: (sidebarSide) => set({ sidebarSide }),
			setSidebarWidth: (width) => set({ sidebarWidth: normaliseWidth(width) }),
		}),
		{
			name: SHELL_STORAGE_KEY,
			// The actions are rebuilt on every boot. Persisting them writes `null`
			// over each one, and `merge` runs with `replace: true`, so rehydration
			// would install those nulls in place of the live functions.
			partialize: (s) => ({ sidebarSide: s.sidebarSide, sidebarWidth: s.sidebarWidth }),
			// `current` must be spread first: this replaces the state rather than
			// patching it, so anything not named here is simply gone.
			merge: (persisted, current) => {
				const stored = persisted as { sidebarSide?: unknown; sidebarWidth?: unknown } | undefined;
				return {
					...current,
					sidebarSide: normaliseSide(stored?.sidebarSide),
					sidebarWidth: normaliseWidth(stored?.sidebarWidth),
				};
			},
		}
	)
);
