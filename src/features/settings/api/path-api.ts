import { toPathStatus, type PathStatus, type PathStatusDTO } from '@/lib/core-store/dtos/v0/path';
import { apiFetch } from '@/lib/transport/api';

export async function getPathStatus(): Promise<PathStatus> {
	return toPathStatus(await apiFetch<PathStatusDTO>('/v0/system/path'));
}

/** Only ever called from an explicit user action: core never edits `PATH` on its own. */
export async function setupPath(): Promise<PathStatus> {
	return toPathStatus(await apiFetch<PathStatusDTO>('/v0/system/path', { method: 'POST' }));
}
