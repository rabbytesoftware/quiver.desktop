import { create } from 'zustand';

import type { ConnectionStatus } from '@/domain/connection';

interface StatusStoreState {
	status: ConnectionStatus;
	setStatus: (status: ConnectionStatus) => void;
}

export const useStatusStore = create<StatusStoreState>((set) => ({
	status: 'starting',
	setStatus: (status) => set({ status }),
}));
