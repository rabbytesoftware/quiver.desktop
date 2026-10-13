import { create } from 'zustand';

export type SettingsTab = 'general' | 'engine' | 'developer';

interface SettingsUIState {
	tab: SettingsTab;
	setTab: (tab: SettingsTab) => void;
}

export const useSettingsUI = create<SettingsUIState>((set) => ({
	tab: 'general',
	setTab: (tab) => set({ tab }),
}));
