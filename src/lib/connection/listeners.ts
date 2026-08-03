import { backend } from '@/lib/transport/backend';

import { useConnectionStore } from './store';

/**
 * `connection://changed` fires on add, remove and rename only —
 * `switch_connection` emits nothing at all — so without the seed a session
 * where nobody touches a host never receives an event and this store stays
 * empty for its whole life.
 */
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
