import { createClock } from './clock';
import { getScenario } from './scenarios';
import type { Emitter, MockWorld, ScenarioName } from './types';
import { versioned } from './types';

/**
 * The cache partition a mock scenario writes under.
 *
 * `ArrowCatalogRecord`'s key is `[connectionId, namespace]` and `getArrowsFor`
 * reads by that index, so giving each scenario its own id means switching
 * scenarios needs NO cache clearing at all — the rows simply sit in a partition
 * nothing is reading. Crowbar had to `db.clear()` four stores on every scenario
 * switch because its cache had no such partition.
 *
 * It also means mock rows can never appear under `local`. That is the property
 * standing between this feature and a bug report titled "all my arrows
 * disappeared".
 */
export function mockConnectionId(scenario: ScenarioName): string {
	return `mock:${scenario}`;
}

export function buildWorld(scenario: ScenarioName, emitter: Emitter): MockWorld {
	const descriptor = getScenario(scenario);
	const data = descriptor.build();

	let counter = 0;

	const world: MockWorld = {
		scenario: descriptor.name,
		connectionId: mockConnectionId(descriptor.name),
		arrows: new Map(data.arrows.map((a) => [versioned(a), a])),
		collections: new Map(data.collections.map((c) => [c.namespace, c])),
		jobs: new Map(),
		cancels: new Map(),
		clock: createClock(),
		emitter,
		// Monotonic rather than random: job ids show up in URLs and in logs, and
		// a run that produces `job-1` every time is a run you can compare against
		// the last one.
		nextId: () => ++counter,
	};

	return world;
}

/** The providers a discovery pass reports. Held here so a job can be rebuilt. */
export function providersFor(scenario: ScenarioName) {
	return getScenario(scenario).build().providers;
}
