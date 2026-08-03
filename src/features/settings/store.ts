import { create } from 'zustand';

export type SettingsTab = 'connections' | 'developer';

interface SettingsUIState {
	open: boolean;
	tab: SettingsTab;
	/** Filters rows across every tab, the way Crowbar's header search does. */
	query: string;
	openSettings: (tab?: SettingsTab) => void;
	closeSettings: () => void;
	setTab: (tab: SettingsTab) => void;
	setQuery: (query: string) => void;
}

export const useSettingsUI = create<SettingsUIState>((set) => ({
	open: false,
	tab: 'connections',
	query: '',

	openSettings: (tab) => set((s) => ({ open: true, tab: tab ?? s.tab })),
	// The query is cleared on close, not kept: reopening to a filtered panel
	// with a search box you have to notice and empty reads as a broken panel.
	closeSettings: () => set({ open: false, query: '' }),
	setTab: (tab) => set({ tab }),
	setQuery: (query) => set({ query }),
}));

/**
 * Whether a row survives the active filter. Ported from Crowbar's
 * `settings-row-search.ts`.
 *
 * Substring, case-insensitive, over label AND description — not fuzzy. Someone
 * typing "fault" wants the fault rows, and a fuzzy matcher that also returns
 * "Default scenario" makes the filter worse than no filter.
 */
export function rowMatchesQuery(query: string, label: string, description?: string): boolean {
	const needle = query.trim().toLowerCase();
	if (!needle) return true;
	return `${label} ${description ?? ''}`.toLowerCase().includes(needle);
}
