import { createClock } from './clock';
import { getScenario } from './scenarios';
import type { Emitter, MockWorld, ScenarioName } from './types';
import { CONFIG_DEFAULTS, versioned } from './types';

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
		config: {
			running: structuredClone(CONFIG_DEFAULTS),
			configured: structuredClone(CONFIG_DEFAULTS),
			corrected: [],
		},
		nextId: () => ++counter,
	};

	return world;
}

export function providersFor(scenario: ScenarioName) {
	return getScenario(scenario).build().providers;
}
