import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import type { Backend, ConnectionsSnapshot } from '@/lib/transport/backend';
import { installBackend } from '@/lib/transport/backend';

import { ALL_ROUTES } from './server/handlers';
import { createRouter } from './server/router';
import { createSocketHub } from './socket';
import { buildWorld, mockConnectionId } from './world/build';
import { getScenario } from './world/scenarios';
import type { MockWorld, ScenarioName } from './world/types';

export interface MockRuntime {
	backend: Backend;
	world: MockWorld;
	/** Stops every fabricated timer and closes every socket. */
	dispose: () => void;
}

/** Going straight to `ready` would skip the Connecting screen entirely. */
const BOOT_MS = 400;

export function createMockBackend(scenario: ScenarioName): MockRuntime {
	const hub = createSocketHub();
	const world = buildWorld(scenario, hub);
	const router = createRouter(ALL_ROUTES);
	const descriptor = getScenario(scenario);

	const connection: ConnectionConfig = {
		id: mockConnectionId(descriptor.name),
		name: `Mock · ${descriptor.label}`,
		kind: 'local',
		api_version: 'v0',
	};

	const snapshot: ConnectionsSnapshot = { connections: [connection], active_id: connection.id };

	const backend: Backend = {
		fetch(path, init) {
			// Never calls `apiBase()`: "no origin" is only an error for a backend
			// that needs one.
			return router.handle(path, init, world);
		},

		openSocket(path) {
			return hub.open(path);
		},

		getConnections() {
			return Promise.resolve(snapshot);
		},

		onCoreStatus(cb) {
			// On a timer, not synchronously: `setupListeners` registers this and
			// THEN probes with `coreIsReachable`, and a `ready` delivered before it
			// returns would race that probe into starting streams twice.
			world.clock.after(0, () => cb('starting' as ConnectionStatus));
			world.clock.after(BOOT_MS, () => cb('ready' as ConnectionStatus));
			return Promise.resolve(() => {});
		},

		onConnectionsChanged() {
			// One connection, and no way to add, remove or rename one. The list
			// arrives through `getConnections`.
			return Promise.resolve(() => {});
		},
	};

	return {
		backend,
		world,
		dispose: () => {
			world.clock.cancelAll();
			hub.closeAll();
		},
	};
}

let installed: MockRuntime | null = null;

/**
 * Returns null rather than throwing: this ships in release builds, where
 * whoever has the flag set may not know it exists and cannot reach the setting
 * that clears it if the app will not start.
 */
export function installMock(scenario: ScenarioName): MockRuntime | null {
	try {
		installed?.dispose();
		installed = createMockBackend(scenario);
		installBackend(installed.backend);
		console.info(`quiver: mock backend active (scenario: ${scenario}). No daemon is being contacted.`);
		return installed;
	} catch (err) {
		console.error('quiver: mock backend failed to start; falling back to the real one', err);
		installed = null;
		return null;
	}
}

/** The live mock, if one is installed. */
export function currentMock(): MockRuntime | null {
	return installed;
}

/**
 * Does NOT put the real backend back: open streams, a seeded cache and a
 * projection full of fabricated arrows all belong to the world being torn down.
 * Turning the mock off is a reload for that reason.
 */
export function disposeMock(): void {
	installed?.dispose();
	installed = null;
}
