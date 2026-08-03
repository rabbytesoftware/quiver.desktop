import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { ScenarioName } from './world/types';

export const MOCK_STORAGE_KEY = 'quiver.mock';

/**
 * One key per route family, not per route.
 *
 * Finer than this and the Developer tab becomes a wall of sliders nobody reads;
 * coarser and "make search fail" also breaks the library, so you cannot tell
 * which empty state you are looking at.
 */
export const FAULT_KEYS = [
	'arrows',
	'arrow-detail',
	'search',
	'discover',
	'collections',
	'collection-detail',
	'runtime',
	'health',
] as const;

export type FaultKey = (typeof FAULT_KEYS)[number];

export const FAULT_LABELS: Record<FaultKey, string> = {
	arrows: 'Arrow catalog',
	'arrow-detail': 'Arrow detail',
	search: 'Search',
	discover: 'Discovery',
	collections: 'Collections',
	'collection-detail': 'Collection detail',
	runtime: 'Runtime actions',
	health: 'Health probe',
};

const NO_FAULTS: Record<FaultKey, number> = Object.fromEntries(FAULT_KEYS.map((k) => [k, 0])) as Record<
	FaultKey,
	number
>;

interface MockState {
	// ── Persisted ────────────────────────────────────────────────────────────
	enabled: boolean;
	scenario: ScenarioName;
	faults: Record<FaultKey, number>;
	/** Whether the Developer tab has been unlocked in a release build. */
	devUnlocked: boolean;

	// ── Ephemeral, per session ───────────────────────────────────────────────
	// Latency and error rate are things you turn on to watch one thing happen,
	// not settings. Surviving a restart, they would be a mystery a week later.
	latency: number;
	errorRate: number;
	unreachable: boolean;

	setScenario: (scenario: ScenarioName) => void;
	setFault: (key: FaultKey, pct: number) => void;
	setLatency: (ms: number) => void;
	setErrorRate: (rate: number) => void;
	setUnreachable: (on: boolean) => void;
	resetFaults: () => void;
	resetChaos: () => void;
	unlockDeveloper: () => void;

	/**
	 * Persist, then reload.
	 *
	 * Which backend is installed is resolved once at boot, before
	 * `setupListeners` and before React renders, so there is no honest way to
	 * swap it live: the streams are already open against the old one and the
	 * cache is already seeded from it. Reloading is the truthful version of what
	 * a live swap would have to do anyway.
	 */
	applyAndReload: (next: { enabled?: boolean; scenario?: ScenarioName }) => void;
}

export const useMockStore = create<MockState>()(
	persist(
		(set, get) => ({
			enabled: false,
			scenario: 'normal',
			faults: { ...NO_FAULTS },
			devUnlocked: false,

			latency: 0,
			errorRate: 0,
			unreachable: false,

			setScenario: (scenario) => set({ scenario }),
			setFault: (key, pct) => set({ faults: { ...get().faults, [key]: pct } }),
			setLatency: (latency) => set({ latency }),
			setErrorRate: (errorRate) => set({ errorRate }),
			setUnreachable: (unreachable) => set({ unreachable }),
			resetFaults: () => set({ faults: { ...NO_FAULTS } }),
			resetChaos: () => set({ latency: 0, errorRate: 0, unreachable: false }),
			unlockDeveloper: () => set({ devUnlocked: true }),

			applyAndReload: (next) => {
				set(next);
				// localStorage is synchronous and zustand/persist writes on `set`,
				// so by the time this line runs the choice is already on disk and
				// `readMockPreference` will see it on the way back up.
				window.location.reload();
			},
		}),
		{
			name: MOCK_STORAGE_KEY,
			partialize: (s) => ({
				enabled: s.enabled,
				scenario: s.scenario,
				faults: s.faults,
				devUnlocked: s.devUnlocked,
			}),
		}
	)
);
