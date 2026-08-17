import { apiFetch } from '@/lib/transport/api';

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

export function patchConfig(patch: unknown): Promise<PatchResult> {
	return apiFetch<PatchResult>('/v0/config', {
		method: 'PATCH',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(patch),
	});
}
