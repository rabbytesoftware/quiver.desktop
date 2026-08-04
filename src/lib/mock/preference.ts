import { MOCK_STORAGE_KEY } from './store';
import { SCENARIO_NAMES, type ScenarioName } from './world/types';

export interface MockPreference {
	enabled: boolean;
	scenario: ScenarioName;
}

const OFF: MockPreference = { enabled: false, scenario: 'normal' };

/**
 * Set by `make dev-mock`. FORCES rather than seeds — the Developer tab disables
 * the switch and says so, because a seeded flag would let the switch appear to
 * work, reload, and come straight back on.
 */
export function mockForcedByEnv(): boolean {
	const flag = import.meta.env.VITE_QUIVER_MOCK;
	return flag === '1' || flag === 'true';
}

/**
 * Reads the raw localStorage key rather than the store: this runs before React
 * renders and before persist's rehydration is guaranteed to have settled.
 *
 * Every failure mode lands on OFF — a corrupt value must not keep someone from
 * reaching their real library.
 */
export function readMockPreference(): MockPreference {
	if (mockForcedByEnv()) {
		const named = import.meta.env.VITE_QUIVER_SCENARIO;
		return {
			enabled: true,
			scenario: SCENARIO_NAMES.includes(named as ScenarioName) ? (named as ScenarioName) : 'normal',
		};
	}

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
			scenario: SCENARIO_NAMES.includes(scenario as ScenarioName) ? (scenario as ScenarioName) : 'normal',
		};
	} catch {
		return OFF;
	}
}
