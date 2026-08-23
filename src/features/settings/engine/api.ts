import { apiFetch, ApiError } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

export interface Rejection {
	key: string;
	message: string;
}

export interface DaemonConfig {
	netbridge: { enabled: boolean; ephemeral_port_start: number; ephemeral_port_end: number };
	logger: { enabled: boolean; level: string };
	// Core's config is wider than this tab renders — `search`, `vault`,
	// `manifold`, `arrows`. They are never read here, but they must survive a
	// round trip, so the type stays open rather than exhaustive.
	[section: string]: unknown;
}

export interface ConfigView {
	running: Omit<DaemonConfig, 'api'>;
	configured: DaemonConfig;
	defaults: DaemonConfig;
	restart_required: string[];
	corrected: Rejection[];
}

export interface PatchResult {
	applied: string[];
	rejected: Rejection[];
}

export function getConfig(): Promise<ConfigView> {
	return apiFetch<ConfigView>('/v0/config');
}

function isPatchResult(data: unknown): data is PatchResult {
	if (typeof data !== 'object' || data === null) return false;
	const candidate = data as Record<string, unknown>;
	return Array.isArray(candidate.applied) && Array.isArray(candidate.rejected);
}

// Core answers a fully-rejected patch with `{ success: false, error: null,
// data: { applied: [], rejected: [...] } }` on a 422 status — confirmed
// against a real daemon (origin/develop@56065f3). `success` reads false even
// though the per-key rejection reasons are real data, not an error, so
// `apiFetch`'s blanket "non-2xx or !success throws" rule would discard every
// rejection reason (it keeps only `body.error`, which is null here). So this
// endpoint reads the envelope itself: a well-formed `{ applied, rejected }`
// payload is treated as data regardless of `success` or HTTP status, and
// `ApiError` is thrown only when the body isn't shaped like a patch result.
export async function patchConfig(patch: unknown): Promise<PatchResult> {
	const res = await backend().fetch('/v0/config', {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(patch),
	});
	const body = (await res.json().catch(() => null)) as {
		success: boolean;
		error: string | null;
		data?: unknown;
	} | null;

	if (isPatchResult(body?.data)) return body.data;
	throw new ApiError(body?.error ?? `${res.status} ${res.statusText}`, res.status);
}
