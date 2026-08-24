import type { MockArrow, MockCandidate, MockCollection, MockProvider, ScenarioName } from '../types';
import { buildExtremeArrows, buildExtremeCollections } from './extreme';
import { NORMAL_ARROWS, NORMAL_COLLECTIONS, NORMAL_DISCOVERABLE, NORMAL_PROVIDERS } from './normal';

export interface ScenarioDataset {
	arrows: MockArrow[];
	collections: MockCollection[];
	providers: MockProvider[];
	discoverable: MockCandidate[];
}

export interface ScenarioDescriptor {
	name: ScenarioName;
	label: string;
	summary: string;
	build: () => ScenarioDataset;
}

export const SCENARIOS: ScenarioDescriptor[] = [
	{
		name: 'normal',
		label: 'Normal',
		summary: '17 arrows · every state · a failed install, a yanked ref, a rate-limited host',
		build: () => ({
			arrows: NORMAL_ARROWS.map(clone),
			collections: NORMAL_COLLECTIONS.map(clone),
			providers: NORMAL_PROVIDERS.map(clone),
			discoverable: NORMAL_DISCOVERABLE.map(clone),
		}),
	},
	{
		name: 'extreme',
		label: 'Extreme',
		summary: '200 arrows · 12 collections · the point where the rail scrolls and search stops being optional',
		build: () => {
			const arrows = buildExtremeArrows();
			return {
				arrows,
				collections: buildExtremeCollections(arrows),
				providers: NORMAL_PROVIDERS.map(clone),
				discoverable: NORMAL_DISCOVERABLE.map(clone),
			};
		},
	},
	{
		name: 'empty',
		label: 'Empty',
		summary: 'A first run — nothing installed, nothing followed, nothing to search',
		build: () => ({ arrows: [], collections: [], providers: [], discoverable: [] }),
	},
];

function clone<T>(value: T): T {
	return structuredClone(value);
}

export function getScenario(name: string): ScenarioDescriptor {
	return SCENARIOS.find((s) => s.name === name) ?? SCENARIOS[0];
}
