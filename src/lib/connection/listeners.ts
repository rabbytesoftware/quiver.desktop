import { listen } from '@tauri-apps/api/event';

import type { ConnectionConfig } from '@/domain/connection';

import { useConnectionStore } from './store';

interface ConnectionChangedPayload {
	connections: ConnectionConfig[];
	active_id:   string;
}

export async function setupConnectionListeners(): Promise<void> {
	await listen<ConnectionChangedPayload>('connection://changed', (e) => {
		useConnectionStore
			.getState()
			.setFromEvent(e.payload.connections, e.payload.active_id);
	});
}
