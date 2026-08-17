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

// Core answers a fully-rejected patch with a *success-shaped* envelope —
// `{ success: true, error: null, data: { applied: [], rejected: [...] } }` —
// on a 422 status, because the per-key rejection reasons are real data, not
// an error. `apiFetch` throws on any non-2xx status and keeps only
// `body.error` (null here), which would discard every rejection reason. So
// this endpoint reads the envelope itself instead of going through `apiFetch`.
export async function patchConfig(patch: unknown): Promise<PatchResult> {
	const res = await backend().fetch('/v0/config', {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(patch),
	});
	const body = (await res.json().catch(() => null)) as {
		success: boolean;
		error: string | null;
		data?: PatchResult;
	} | null;

	if (body?.success) return body.data as PatchResult;
	throw new ApiError(body?.error ?? `${res.status} ${res.statusText}`, res.status);
}
