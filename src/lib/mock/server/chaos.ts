import { type FaultKey, useMockStore } from '../store';
import { fail, unreachable } from './envelope';

export type Rng = () => number;

export interface ChaosSettings {
	latency: number;
	errorRate: number;
	unreachable: boolean;
	faults: Record<FaultKey, number>;
}

function currentSettings(): ChaosSettings {
	const { latency, errorRate, unreachable: down, faults } = useMockStore.getState();
	return { latency, errorRate, unreachable: down, faults };
}

export function shouldFault(key: FaultKey, rng: Rng = Math.random, settings = currentSettings()): boolean {
	return rng() * 100 < (settings.faults[key] ?? 0);
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

export async function applyChaos(key: FaultKey, rng: Rng = Math.random): Promise<Response | null> {
	const settings = currentSettings();

	if (settings.latency > 0) await sleep(settings.latency);

	if (settings.unreachable) return unreachable();

	if (settings.errorRate > 0 && rng() * 100 < settings.errorRate) {
		return fail('simulated failure (mock error rate)', 500);
	}

	if (shouldFault(key, rng, settings)) {
		return fail(`simulated failure (mock fault: ${key})`, 500);
	}

	return null;
}
