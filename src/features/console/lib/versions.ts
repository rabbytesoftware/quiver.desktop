import { apiFetch } from '@/lib/transport/api';

/** The console capability `GET /versions` advertises in `features`. */
export const CONSOLE_FEATURE = 'console.v1';

/** `GET /versions`, as this client reads it. Everything beyond `version` is optional on the wire. */
export interface CoreVersions {
	version: string;
	buildId: string;
	commit: string;
	builtAt: string;
	channel: string;
	features: string[];
}

function str(value: unknown): string {
	return typeof value === 'string' ? value : '';
}

/** A daemon that predates the console sends no `features`, which reads as none. */
export function parseVersions(data: unknown): CoreVersions | null {
	if (typeof data !== 'object' || data === null) return null;
	const d = data as Record<string, unknown>;
	return {
		version: str(d.version),
		buildId: str(d.build_id),
		commit: str(d.commit),
		builtAt: str(d.built_at),
		channel: str(d.channel),
		features: Array.isArray(d.features) ? d.features.filter((f): f is string => typeof f === 'string') : [],
	};
}

export function supportsConsole(versions: CoreVersions | null): boolean {
	return versions?.features.includes(CONSOLE_FEATURE) ?? false;
}

export async function fetchCoreVersions(): Promise<CoreVersions | null> {
	return parseVersions(await apiFetch<unknown>('/versions'));
}
