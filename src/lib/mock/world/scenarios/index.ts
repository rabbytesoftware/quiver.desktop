import type { MockArrow, MockCollection, MockProvider, ScenarioName } from '../types';
import { buildExtremeArrows, buildExtremeCollections } from './extreme';
import { NORMAL_ARROWS, NORMAL_COLLECTIONS, NORMAL_PROVIDERS } from './normal';

export interface ScenarioDataset {
	arrows: MockArrow[];
	collections: MockCollection[];
	/** What a discovery pass reports back, per host. */
	providers: MockProvider[];
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
			// Cloned, not shared. The world MUTATES these — install rewrites state,
			// follow flips a flag — and handing out the module-level literal would
			// let one session's install leak into the next `buildWorld`, which is
			// exactly the cross-test contamination the seeded RNG exists to avoid.
			arrows: NORMAL_ARROWS.map(clone),
			collections: NORMAL_COLLECTIONS.map(clone),
			providers: NORMAL_PROVIDERS.map(clone),
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
			};
		},
	},
	{
		name: 'empty',
		label: 'Empty',
		summary: 'A first run — nothing installed, nothing followed, nothing to search',
		build: () => ({ arrows: [], collections: [], providers: [] }),
	},
];

function clone<T>(value: T): T {
	return structuredClone(value);
}

/**
 * Falls back to `normal` rather than throwing.
 *
 * A stale `quiver.mock` in localStorage naming a scenario that no longer exists
 * would otherwise brick the app at boot for anyone who had it set — including
 * in a release build, where they may have no idea the flag is there and no way
 * to reach the setting that clears it.
 */
export function getScenario(name: string): ScenarioDescriptor {
	return SCENARIOS.find((s) => s.name === name) ?? SCENARIOS[0];
}
