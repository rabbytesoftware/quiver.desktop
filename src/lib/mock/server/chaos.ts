import { type FaultKey, useMockStore } from '../store';
import { fail, unreachable } from './envelope';

/** Injectable so tests are deterministic. Crowbar's equivalent calls Math.random() inline. */
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

/**
 * Everything that can go wrong before a handler is even consulted, in the order
 * a real stack would hit it.
 *
 * Unreachable comes FIRST and is not a probability: a daemon that is down is
 * down for every request, and expressing it as a 100% error rate would produce
 * daemon-shaped 500s instead — a completely different failure the app is
 * supposed to distinguish. This one carries the proxy marker, so `apiFetch`
 * retries it eight times with backoff and the app lands on Disconnected the
 * same way it would against a real dead socket.
 *
 * Returns the response to send, or null to let the route run.
 */
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
