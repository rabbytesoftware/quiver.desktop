import { backend } from '@/lib/transport/backend';

import { useConnectionStore } from './store';

export async function setupConnectionListeners(): Promise<void> {
	await backend().onConnectionsChanged(({ connections, active_id }) => {
		useConnectionStore.getState().setFromEvent(connections, active_id);
	});

	try {
		const { connections, active_id } = await backend().getConnections();
		useConnectionStore.getState().setFromEvent(connections, active_id);
	} catch (err) {
		console.error('connection: failed to read the initial connection list', err);
	}
}
