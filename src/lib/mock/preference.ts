import { MOCK_STORAGE_KEY } from './store';
import { SCENARIO_NAMES, type ScenarioName } from './world/types';

export interface MockPreference {
	enabled: boolean;
	scenario: ScenarioName;
}

const OFF: MockPreference = { enabled: false, scenario: 'normal' };

/**
 * Reads the same localStorage key `useMockStore` persists to, WITHOUT importing
 * the store's hydrated state.
 *
 * `main.tsx` has to choose a backend before `setupListeners` runs and before
 * React renders — earlier than any component could subscribe to a store, and
 * earlier than zustand/persist's own rehydration is guaranteed to have settled.
 * Parsing the raw value is the only reading available that early, and it is
 * exact: `persist` writes `{state, version}` as JSON to localStorage
 * synchronously, so what is on disk at boot is what was last set.
 *
 * Every failure mode lands on OFF, deliberately. A corrupt or half-written value
 * must not be able to keep someone from reaching their real library, and in a
 * release build — where this ships — they may not know the flag exists.
 */
export function readMockPreference(): MockPreference {
	try {
		const raw = localStorage.getItem(MOCK_STORAGE_KEY);
		if (!raw) return OFF;

		const parsed: unknown = JSON.parse(raw);
		const state = (parsed as { state?: unknown } | null)?.state;
		if (typeof state !== 'object' || state === null) return OFF;

		const { enabled, scenario } = state as { enabled?: unknown; scenario?: unknown };
		if (enabled !== true) return OFF;

		return {
			enabled: true,
			// A scenario that no longer exists falls back rather than throwing —
			// same reasoning as `getScenario`, one layer earlier.
			scenario: SCENARIO_NAMES.includes(scenario as ScenarioName) ? (scenario as ScenarioName) : 'normal',
		};
	} catch {
		return OFF;
	}
}
