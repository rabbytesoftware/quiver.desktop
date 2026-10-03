import { ApiError } from '@/lib/transport/api';

export type UpdateState = 'upToDate' | 'available' | 'downloading' | 'staged' | 'restarting' | 'error';

export type UpdateErrorKind = 'busy' | 'rate_limited' | 'offline' | 'unverifiable' | 'unavailable' | 'failed';

export interface UpdateError {
	kind: UpdateErrorKind;
	/** Core's own wording, shown under the sentence for the kind. */
	message: string;
}

interface UpdateSignals {
	restarting: boolean;
	running: boolean;
	error: UpdateError | null;
	pending: boolean;
	available: boolean;
}

/**
 * Restarting outranks everything: the daemon is mid-handover and every other
 * signal is about to be replaced by what the new process reports. A running
 * update outranks an error because starting again is what clears one.
 */
export function deriveUpdateState({ restarting, running, error, pending, available }: UpdateSignals): UpdateState {
	if (restarting) return 'restarting';
	if (running) return 'downloading';
	if (error) return 'error';
	if (pending) return 'staged';
	if (available) return 'available';
	return 'upToDate';
}

const BUSY_STATUSES = [409, 422];
const OFFLINE_STATUSES = [502, 503, 504];

const RELEASE_UNRESOLVED = /release unresolved: (\w+)/;

/**
 * Whether core refused to start an update because the release it was to come
 * from could not be resolved. Core answers these with 422, the status it also
 * uses for a state violation, so the status alone cannot tell them apart.
 */
export function isReleaseFailure(err: unknown): boolean {
	return err instanceof ApiError && RELEASE_UNRESOLVED.test(err.message);
}

function releaseFailureKind(reason: string): UpdateErrorKind {
	switch (reason) {
		case 'rate_limited':
			return 'rate_limited';
		case 'offline':
			return 'offline';
		case 'unverifiable':
			return 'unverifiable';
		default:
			return 'unavailable';
	}
}

export function classifyUpdateError(err: unknown): UpdateError {
	if (err instanceof ApiError) {
		const release = RELEASE_UNRESOLVED.exec(err.message);
		if (release) return { kind: releaseFailureKind(release[1]), message: err.message };
		if (BUSY_STATUSES.includes(err.status)) return { kind: 'busy', message: err.message };
		if (err.status === 429) return { kind: 'rate_limited', message: err.message };
		if (OFFLINE_STATUSES.includes(err.status)) return { kind: 'offline', message: err.message };
	}
	return { kind: 'failed', message: err instanceof Error ? err.message : String(err) };
}

/**
 * Whether a failed `activate` request means the daemon went away rather than
 * refused. Activation relaunches the daemon, so the connection can close before
 * the reply is written; that is the success path, not an error.
 */
export function isConnectionDrop(err: unknown): boolean {
	if (err instanceof ApiError) return err.status === 502 || err.status === 504;
	return true;
}
