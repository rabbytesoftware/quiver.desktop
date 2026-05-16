import { create } from 'zustand';

import type { ConnectionConfig } from '@/domain/connection';

interface ConnectionState {
	connections: ConnectionConfig[];
	activeId: string;
	setFromEvent: (connections: ConnectionConfig[], activeId: string) => void;
}

export const useConnectionStore = create<ConnectionState>((set) => ({
	connections: [],
	activeId: 'local',
	setFromEvent: (connections, activeId) => set({ connections, activeId }),
}));
