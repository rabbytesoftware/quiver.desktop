import type { PathStatus } from '@/lib/core-store/dtos/v0/path';

export type PathState = 'ready' | 'restart' | 'setup';

/**
 * `ready`: the daemon already sees the directory on `PATH`. `restart`: it is
 * configured but only terminals opened from now on will have it. `setup`:
 * neither, so the user has to ask for it.
 */
export function pathState(status: PathStatus): PathState {
	if (status.onPath) return 'ready';
	return status.configured ? 'restart' : 'setup';
}
