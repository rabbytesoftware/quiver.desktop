import { backend } from '@/lib/transport/backend';

import { useConnectionStore } from './store';

/**
 * Populates the connections UI, and keeps it in step.
 *
 * The SEED is not decoration. `connection://changed` fires on add, remove and
 * rename only — `switch_connection` emits nothing at all (a known Rust-side
 * gap, documented at length in `core-store/listeners`) — so a session where
 * nobody adds a host never received a single event, and the store sat at its
 * empty initial value forever. That was invisible while nothing rendered the
 * list; it is the first thing you notice once something does.
 *
 * Routed through the backend rather than `invoke`/`listen` directly for the
 * same reason everything else is: outside the Tauri shell there is no IPC to
 * call, and this is invoked un-awaited from `main.tsx`, so a rejection would
 * surface as an unhandled rejection during boot rather than as anything
 * anyone could act on.
 */
export async function setupConnectionListeners(): Promise<void> {
	// Subscribed BEFORE the seed is read, so a mutation landing while that read
	// is in flight is not dropped. The seed then overwrites it with a newer
	// snapshot, which is the correct resolution either way.
	await backend().onConnectionsChanged(({ connections, active_id }) => {
		useConnectionStore.getState().setFromEvent(connections, active_id);
	});

	try {
		const { connections, active_id } = await backend().getConnections();
		useConnectionStore.getState().setFromEvent(connections, active_id);
	} catch (err) {
		// A failed seed leaves the list empty rather than breaking the app. The
		// subscription above is already live, so the first add/remove/rename
		// still fills it in.
		console.error('connection: failed to read the initial connection list', err);
	}
}
