import { create } from 'zustand';

import type { SwitchStep } from '@/features/arrow-details/lib/switch-selector';

/** A switch that stopped after the new identity was registered -- the steps before `step` happened, none after. */
export interface SwitchFailure {
	step: Exclude<SwitchStep, 'register'>;
	from: string;
	to: string;
	reason: string;
}

interface SelectorSwitchState {
	/** The switch in flight, if any. */
	active: { from: string; to: string } | null;
	failure: SwitchFailure | null;
	begin: (from: string, to: string) => void;
	end: (failure?: SwitchFailure) => void;
	dismiss: () => void;
}

/**
 * Outlives the page a switch started on: the page moves to the new identity
 * as soon as it is registered, so what happens after -- and how it ended --
 * cannot live in that page's own state.
 */
export const useSelectorSwitchStore = create<SelectorSwitchState>((set) => ({
	active: null,
	failure: null,
	begin: (from, to) => set({ active: { from, to }, failure: null }),
	end: (failure) => set({ active: null, failure: failure ?? null }),
	dismiss: () => set({ failure: null }),
}));
