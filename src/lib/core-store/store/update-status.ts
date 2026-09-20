import { create } from 'zustand';

export interface CoreUpdateStatus {
	outdated: boolean;
	recommended_ref?: string;
}

interface CoreUpdateStatusStoreState {
	status: CoreUpdateStatus;
	setStatus: (status: CoreUpdateStatus) => void;
}

export const useCoreUpdateStatusStore = create<CoreUpdateStatusStoreState>((set) => ({
	status: { outdated: false },
	setStatus: (status) => set({ status }),
}));
