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
	dispose: () => void;
}

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

	// A saved-but-inactive remote, so Remote Control has more than "Local" to
	// show under the mock -- its list, switcher, and populated-vs-empty
	// states all need a second connection to exercise. Named and hosted the
	// same way `connection` above is: unmistakably fake, never a real host.
	const remote: ConnectionConfig = {
		id: 'mock:home-lab',
		name: 'Mock · Home Lab',
		kind: 'remote',
		url: 'http://mock.home-lab.local:7420',
		api_version: 'v0',
	};

	const snapshot: ConnectionsSnapshot = {
		connections: [connection, remote],
		active_id: connection.id,
	};

	const backend: Backend = {
		fetch(path, init) {
			return router.handle(path, init, world);
		},

		openSocket(path) {
			return hub.open(path);
		},

		getConnections() {
			return Promise.resolve(snapshot);
		},

		onCoreStatus(cb) {
			world.clock.after(0, () => cb('starting' as ConnectionStatus));
			world.clock.after(BOOT_MS, () => cb('ready' as ConnectionStatus));
			return Promise.resolve(() => {});
		},

		onConnectionsChanged() {
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

export function currentMock(): MockRuntime | null {
	return installed;
}

export function disposeMock(): void {
	installed?.dispose();
	installed = null;
}
