import type { Rejection } from '@/features/settings/api/engine-api';

// Automatic arrow registration is quiver.core's Fletcher: it builds a manifest
// for a repository that ships none. It is the only switch; the search pass
// that surfaces such repositories is not flagged.
const FLETCHER_KEY = 'manifold.fletcher';

interface AutoRegisterSource {
	manifold?: unknown;
	[section: string]: unknown;
}

/** On only when `manifold.fletcher.enabled` is true. A daemon that predates the key omits it, and that reads as off. */
export function isAutoRegisterOn(config: AutoRegisterSource): boolean {
	const { manifold } = config;
	if (typeof manifold !== 'object' || manifold === null) return false;
	const fletcher = (manifold as Record<string, unknown>).fletcher;
	if (typeof fletcher !== 'object' || fletcher === null) return false;
	return (fletcher as Record<string, unknown>).enabled === true;
}

/** The config patch that turns Fletcher on or off. */
export function autoRegisterPatch(on: boolean) {
	return { manifold: { fletcher: { enabled: on } } };
}

/** The patch that puts Fletcher back to whatever the daemon defaults to. */
export function autoRegisterResetPatch() {
	return { manifold: { fletcher: { enabled: null } } };
}

/**
 * Why the daemon refused the switch, if it did. An older daemon does not know
 * the key and rejects it by name, at whatever depth it stopped reading.
 */
export function autoRegisterRejection(rejected: Rejection[]): string | undefined {
	return rejected.find(
		(r) => r.key === FLETCHER_KEY || r.key.startsWith(`${FLETCHER_KEY}.`) || FLETCHER_KEY.startsWith(`${r.key}.`)
	)?.message;
}
