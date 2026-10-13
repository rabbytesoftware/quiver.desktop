import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { SidebarSide } from '@/features/shell/lib/geometry';

export const SIDEBAR_MIN = 160;

export const SIDEBAR_MAX = 320;

export const SIDEBAR_DEFAULT = 246;

export const SHELL_STORAGE_KEY = 'quiver.shell';

export interface ShellState {
	sidebarSide: SidebarSide;
	sidebarWidth: number;
	setSidebarSide: (side: SidebarSide) => void;
	setSidebarWidth: (width: number) => void;
}

export function normaliseSide(value: unknown): SidebarSide {
	return value === 'right' ? 'right' : 'left';
}

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
			partialize: (s) => ({ sidebarSide: s.sidebarSide, sidebarWidth: s.sidebarWidth }),
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
