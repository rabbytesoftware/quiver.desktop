import { create } from 'zustand';

export type SettingsTab = 'general' | 'connections' | 'developer';

interface SettingsUIState {
	tab: SettingsTab;
	query: string;
	setTab: (tab: SettingsTab) => void;
	setQuery: (query: string) => void;
}

export const useSettingsUI = create<SettingsUIState>((set) => ({
	tab: 'general',
	query: '',

	setTab: (tab) => set({ tab }),
	setQuery: (query) => set({ query }),
}));

export function rowMatchesQuery(query: string, label: string, description?: string): boolean {
	const needle = query.trim().toLowerCase();
	if (!needle) return true;
	return `${label} ${description ?? ''}`.toLowerCase().includes(needle);
}
