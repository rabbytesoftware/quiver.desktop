import { create } from 'zustand';

import { invoke } from '@tauri-apps/api/core';

import type { ConnectionConfig } from '@/domain/connection';
import { localConnection } from '@/domain/connection';

interface ConnectionStoreState {
	connections: ConnectionConfig[];
	activeId: string;
	setConnections: (connections: ConnectionConfig[]) => void;
	setActiveId: (id: string) => void;
}

export const useConnectionStore = create<ConnectionStoreState>((set) => ({
	connections: [localConnection()],
	activeId: 'local',
	setConnections: (connections) => set({ connections }),
	setActiveId: (activeId) => set({ activeId }),
}));

export async function loadConnections(): Promise<void> {
	const connections = await invoke<ConnectionConfig[]>('list_connections');
	useConnectionStore.getState().setConnections(connections);
}
