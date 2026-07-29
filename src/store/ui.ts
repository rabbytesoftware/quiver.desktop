import { create } from 'zustand';

export type NavMode = 'home' | 'arrow' | 'search';

const SIDEBAR_MIN = 120;
const SIDEBAR_MAX = 320;

interface UIStore {
	sidebarWidth: number;
	selectedNamespace: string | null;
	navMode: NavMode;
	setSidebarWidth: (width: number) => void;
	selectArrow: (namespace: string) => void;
	setNavMode: (mode: NavMode) => void;
	goHome: () => void;
}

export const useUIStore = create<UIStore>((set) => ({
	sidebarWidth: 200,
	selectedNamespace: null,
	navMode: 'home',

	setSidebarWidth: (width) => set({ sidebarWidth: Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, width)) }),

	selectArrow: (namespace) => set({ selectedNamespace: namespace, navMode: 'arrow' }),

	setNavMode: (mode) => set({ navMode: mode }),

	goHome: () => set({ selectedNamespace: null, navMode: 'home' }),
}));
