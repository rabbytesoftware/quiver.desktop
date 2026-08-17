import { create } from 'zustand';

import { getConfig, patchConfig, type ConfigView, type Rejection } from './api';

interface EngineState {
	view: ConfigView | null;
	rejected: Rejection[];
	loading: boolean;
	error: string | null;
	load: () => Promise<void>;
	patch: (patch: unknown) => Promise<void>;
}

function errorMessage(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

export const useEngineStore = create<EngineState>((set) => ({
	view: null,
	rejected: [],
	loading: false,
	error: null,

	load: async () => {
		set({ loading: true, error: null });
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
		set({ loading: true, error: null });
		try {
			const result = await patchConfig(patch);
			set({ rejected: result.rejected, view: await getConfig(), loading: false });
		} catch (err) {
			set({ error: errorMessage(err), loading: false });
		}
	},
}));
