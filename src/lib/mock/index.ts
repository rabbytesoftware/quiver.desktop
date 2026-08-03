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
	/** Stops every fabricated timer and closes every socket. Tests, and teardown. */
	dispose: () => void;
}

/**
 * How long the fake daemon "takes to start".
 *
 * Not decoration. `core://status` going straight to `ready` would skip the
 * Connecting screen entirely, and that screen is one of the ones this exists to
 * let us build. Short enough not to be a wait, long enough to see.
 */
const BOOT_MS = 400;

export function createMockBackend(scenario: ScenarioName): MockRuntime {
	const hub = createSocketHub();
	const world = buildWorld(scenario, hub);
	const router = createRouter(ALL_ROUTES);
	const descriptor = getScenario(scenario);

	const connection: ConnectionConfig = {
		id: mockConnectionId(descriptor.name),
		// Named so it is unmistakable in the host switcher. Half of "all my
		// arrows disappeared" is prevented in the chrome; this is the other half.
		name: `Mock · ${descriptor.label}`,
		kind: 'local',
		api_version: 'v0',
	};

	const snapshot: ConnectionsSnapshot = { connections: [connection], active_id: connection.id };

	const backend: Backend = {
		fetch(path, init) {
			// Never calls `apiBase()`. A mock genuinely has no origin, and that is
			// not the fault `apiBase` exists to refuse — "no origin" is only an
			// error for a backend that needs one.
			return router.handle(path, init, world);
		},

		openSocket(path) {
			return hub.open(path);
		},

		getConnections() {
			return Promise.resolve(snapshot);
		},

		onCoreStatus(cb) {
			// Announced on a timer rather than synchronously, for the same reason
			// Rust does: `setupListeners` registers this handler and THEN probes
			// with `coreIsReachable`. A `ready` delivered before the caller
			// returned would race that probe, and both routes would start streams
			// for the same generation — the exact duplication `adoptRunningCore`'s
			// `startPending` guard exists to prevent.
			world.clock.after(0, () => cb('starting' as ConnectionStatus));
			world.clock.after(BOOT_MS, () => cb('ready' as ConnectionStatus));
			return Promise.resolve(() => {});
		},

		onConnectionsChanged() {
			// The mock has exactly one connection and no way to add, remove or
			// rename one, so this event has nothing to report. The initial list
			// arrives through `getConnections`, which is what actually populates
			// the UI.
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
 * Swap the app onto a fabricated daemon. Called from `main.tsx` BEFORE
 * `setupListeners` and before React renders.
 *
 * Returns null if the mock could not be built. It is deliberately impossible
 * for a broken fixture to brick the app: this ships in release builds, where
 * whoever has the flag set may not know it exists and certainly cannot reach
 * the setting that clears it if the app will not start.
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

/** The live mock, if one is installed. The chrome indicator reads this. */
export function currentMock(): MockRuntime | null {
	return installed;
}

/**
 * Retire the installed mock: stop its timers, close its sockets, forget it.
 *
 * Does NOT put the real backend back. Everything downstream — open streams, a
 * seeded cache, a projection full of fabricated arrows — belongs to the world
 * being torn down here, and swapping a live daemon in underneath it would leave
 * the app in a state neither backend describes. Turning the mock off is a
 * reload for exactly that reason.
 */
export function disposeMock(): void {
	installed?.dispose();
	installed = null;
}
