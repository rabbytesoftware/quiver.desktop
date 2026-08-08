import { create } from 'zustand';

export type SettingsTab = 'general' | 'connections' | 'developer';

interface SettingsUIState {
	open: boolean;
	tab: SettingsTab;
	/** Filters rows across every tab. */
	query: string;
	openSettings: (tab?: SettingsTab) => void;
	closeSettings: () => void;
	setTab: (tab: SettingsTab) => void;
	setQuery: (query: string) => void;
}

export const useSettingsUI = create<SettingsUIState>((set) => ({
	open: false,
	tab: 'general',
	query: '',

	openSettings: (tab) => set((s) => ({ open: true, tab: tab ?? s.tab })),
	// Cleared on close: reopening to a filtered panel reads as a broken one.
	closeSettings: () => set({ open: false, query: '' }),
	setTab: (tab) => set({ tab }),
	setQuery: (query) => set({ query }),
}));

/** Substring over label and description, not fuzzy: someone typing "fault"
 *  wants the fault rows, not "Default scenario".
 *
 *  Matches the ALREADY-TRANSLATED strings the row was handed, so the search
 *  works in whatever language is on screen. Matching keys instead would only
 *  ever find rows by their English name. `toLowerCase` without a locale is
 *  deliberate here — `toLocaleLowerCase` would apply Turkish dotless-i rules to
 *  a query typed by a Turkish user against English text and stop matching it. */
export function rowMatchesQuery(query: string, label: string, description?: string): boolean {
	const needle = query.trim().toLowerCase();
	if (!needle) return true;
	return `${label} ${description ?? ''}`.toLowerCase().includes(needle);
}
