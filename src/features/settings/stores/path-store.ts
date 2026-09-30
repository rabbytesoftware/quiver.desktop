import { create } from 'zustand';

import { getPathStatus, setupPath } from '@/features/settings/api/path-api';
import type { PathStatus } from '@/lib/core-store/dtos/v0/path';
import { isNotFoundError } from '@/lib/transport/api';

interface PathState {
	status: PathStatus | null;
	loading: boolean;
	settingUp: boolean;
	// A daemon that predates `/v0/system/path` answers 404; the panel hides
	// the entry rather than showing a failure the user cannot act on.
	unavailable: boolean;
	// Owned by `load`: there is no status to show, so the row offers a retry.
	error: string | null;
	// Owned by `setup`: a status is already on screen, so this renders inline.
	setupError: string | null;
	load: () => Promise<void>;
	setup: () => Promise<void>;
}

function errorMessage(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

export const usePathStore = create<PathState>((set) => ({
	status: null,
	loading: true,
	settingUp: false,
	unavailable: false,
	error: null,
	setupError: null,

	load: async () => {
		set({ loading: true, error: null, setupError: null, unavailable: false });
		try {
			set({ status: await getPathStatus(), loading: false });
		} catch (err) {
			if (isNotFoundError(err)) set({ unavailable: true, loading: false });
			else set({ error: errorMessage(err), loading: false });
		}
	},

	// The response is the updated status, so it is used as-is; no re-read.
	setup: async () => {
		set({ settingUp: true, setupError: null });
		try {
			set({ status: await setupPath(), settingUp: false });
		} catch (err) {
			set({ setupError: errorMessage(err), settingUp: false });
		}
	},
}));
