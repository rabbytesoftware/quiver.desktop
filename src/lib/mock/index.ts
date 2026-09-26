import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';
import type { ReleaseResolveError } from '@/domain/release';
import { currentPlatform } from '@/lib/platform';
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

		// `null`, and not a plausible-looking `stable-*` string: the build tag
		// is a property of the BINARY, not of the daemon this stands in for,
		// and anything running the mock is by definition an untagged dev
		// build. Handing one back would make `announceSelf` claim a release
		// ref no mock scenario can make true.
		getBuildTag() {
			return Promise.resolve(null);
		},

		// The mock runs inside the same webview a real build would, so the UA
		// guess is the honest answer here too -- there is no native side behind
		// the mock to ask instead, and nothing in a mock scenario needs the
		// Apple-Silicon-accurate answer the real command exists for.
		getPlatform() {
			return Promise.resolve(currentPlatform());
		},

		// Rejected, not answered with a plausible asset. The mock stands in
		// for a daemon, not for github.com, and nothing in a mock scenario
		// can make a release URL real -- handing one back would let a mock
		// run start an update that then fetches a 404. `unsupported_platform`
		// is the honest kind: a browser is not a platform Quiver publishes a
		// bundle for, which is exactly the situation.
		resolveReleaseAsset() {
			return Promise.reject({
				kind: 'unsupported_platform',
				detail: 'the mock backend has no release to resolve',
			} satisfies ReleaseResolveError);
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
