import { backend } from '@/lib/transport/backend';

import { useConnectionStore } from './store';

/**
 * Keeps the connections UI in step with add/remove/rename.
 *
 * Routed through the backend rather than `listen` directly for the same reason
 * everything else is: outside the Tauri shell there is no event bus to listen
 * on, and this is called un-awaited from `main.tsx`, so a rejected subscription
 * would surface as an unhandled rejection during boot rather than as anything
 * anyone could act on.
 *
 * Note what does NOT arrive here: `switch_connection` emits nothing (a known
 * Rust-side gap, documented in `core-store/listeners`), so this event is only
 * ever the three mutations.
 */
export async function setupConnectionListeners(): Promise<void> {
	await backend().onConnectionsChanged(({ connections, active_id }) => {
		useConnectionStore.getState().setFromEvent(connections, active_id);
	});
}
