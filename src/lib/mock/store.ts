import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { ScenarioName } from './world/types';

export const MOCK_STORAGE_KEY = 'quiver.mock';

export const FAULT_KEYS = [
	'arrows',
	'arrow-detail',
	'search',
	'discover',
	'collections',
	'collection-detail',
	'runtime',
	'health',
	'config',
] as const;

export type FaultKey = (typeof FAULT_KEYS)[number];

const NO_FAULTS: Record<FaultKey, number> = Object.fromEntries(FAULT_KEYS.map((k) => [k, 0])) as Record<
	FaultKey,
	number
>;

interface MockState {
	enabled: boolean;
	scenario: ScenarioName;
	faults: Record<FaultKey, number>;
	devUnlocked: boolean;

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
