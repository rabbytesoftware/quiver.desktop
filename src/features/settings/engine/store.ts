import { create } from 'zustand';

import { getConfig, patchConfig, type ConfigView, type Rejection } from './api';

interface EngineState {
	view: ConfigView | null;
	rejected: Rejection[];
	loading: boolean;
	// Owned by `load` alone: a failure here means there is nothing to show at
	// all, so `engine.tsx` replaces the whole panel with a retry screen.
	error: string | null;
	// Owned by `patch` alone: a failure here happens with a perfectly good
	// `view` already on screen, so `engine.tsx` renders it inline, next to the
	// controls, instead of tearing the panel down. Keeping it separate from
	// `error` is what stops a momentary daemon hiccup on one toggle from
	// wiping out every row in the panel.
	patchError: string | null;
	load: () => Promise<void>;
	patch: (patch: unknown) => Promise<void>;
}

function errorMessage(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

export const useEngineStore = create<EngineState>((set) => ({
	view: null,
	rejected: [],
	loading: true,
	error: null,
	patchError: null,

	load: async () => {
		// Also clears `rejected`: `Tabs.Panel` unmounts inactive panels, so
		// leaving Engine and coming back re-runs `load()` while a rejection
		// from the previous visit is still sitting in the store. Without this
		// a row can go on reporting a value as refused long after the field
		// holding it was reverted to something valid.
		set({ loading: true, error: null, rejected: [] });
		try {
			set({ view: await getConfig(), loading: false });
		} catch (err) {
			set({ error: errorMessage(err), loading: false });
		}
	},

	// Re-reads after a patch rather than merging the response in: the daemon
	// owns `restart_required` and `corrected`, and recomputing either here
	// would be guessing at what it already told us.
	patch: async (patch) => {
		set({ loading: true, patchError: null });
		try {
			const result = await patchConfig(patch);
			set({ rejected: result.rejected, view: await getConfig(), loading: false });
		} catch (err) {
			set({ patchError: errorMessage(err), loading: false });
		}
	},
}));
