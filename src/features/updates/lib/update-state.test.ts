import { describe, expect, it } from 'vitest';

import { ApiError } from '@/lib/transport/api';

import { classifyUpdateError, deriveUpdateState, isConnectionDrop, isReleaseFailure } from './update-state';

const IDLE = { restarting: false, running: false, error: null, pending: false, available: false };

describe('deriveUpdateState', () => {
	it('is upToDate when nothing is ahead, staged or underway', () => {
		expect(deriveUpdateState(IDLE)).toBe('upToDate');
	});

	it('is available when a newer version is ahead', () => {
		expect(deriveUpdateState({ ...IDLE, available: true })).toBe('available');
	});

	it('is staged when an update waits for a restart, even with something newer ahead', () => {
		expect(deriveUpdateState({ ...IDLE, pending: true, available: true })).toBe('staged');
	});

	it('is downloading while the update runs', () => {
		expect(deriveUpdateState({ ...IDLE, available: true, running: true })).toBe('downloading');
	});

	it('shows an error over what was available or staged before it', () => {
		const error = { kind: 'failed' as const, message: 'boom' };
		expect(deriveUpdateState({ ...IDLE, available: true, error })).toBe('error');
		expect(deriveUpdateState({ ...IDLE, pending: true, error })).toBe('error');
	});

	it('is restarting over everything else, until the new version reports', () => {
		const error = { kind: 'failed' as const, message: 'boom' };
		expect(deriveUpdateState({ ...IDLE, restarting: true, pending: true, running: true, error })).toBe(
			'restarting'
		);
	});

	it('lets a running update win over a stale error', () => {
		const error = { kind: 'failed' as const, message: 'boom' };
		expect(deriveUpdateState({ ...IDLE, running: true, error })).toBe('downloading');
	});
});

describe('classifyUpdateError', () => {
	it.each([
		[409, 'busy'],
		[422, 'busy'],
		[429, 'rate_limited'],
		[502, 'offline'],
		[503, 'offline'],
		[504, 'offline'],
		[500, 'failed'],
		[404, 'failed'],
	])('maps status %i to %s and keeps the message', (status, kind) => {
		expect(classifyUpdateError(new ApiError('nope', status))).toEqual({ kind, message: 'nope' });
	});

	const RELEASE_UNRESOLVED = (kind: string) =>
		`begin update: resolve QUIVER_RELEASE_ASSET_URL from release x@v2: release unresolved: ${kind}: manifold: nope`;

	it.each([
		['unverifiable', 'unverifiable'],
		['no_release', 'unavailable'],
		['no_asset', 'unavailable'],
		['unsupported_platform', 'unavailable'],
	])('reads a 422 for release %s as %s, never as a busy daemon', (releaseKind, kind) => {
		const message = RELEASE_UNRESOLVED(releaseKind);
		expect(classifyUpdateError(new ApiError(message, 422))).toEqual({ kind, message });
	});

	it('keeps rate limits and outages on their own kinds when the message is typed', () => {
		expect(classifyUpdateError(new ApiError(RELEASE_UNRESOLVED('rate_limited'), 429)).kind).toBe('rate_limited');
		expect(classifyUpdateError(new ApiError(RELEASE_UNRESOLVED('offline'), 502)).kind).toBe('offline');
	});

	it('reads a thrown Error as a failure with its message', () => {
		expect(classifyUpdateError(new Error('socket hang up'))).toEqual({ kind: 'failed', message: 'socket hang up' });
	});

	it('reads a thrown non-Error as a failure with its text', () => {
		expect(classifyUpdateError('weird')).toEqual({ kind: 'failed', message: 'weird' });
	});
});

describe('isReleaseFailure', () => {
	it("recognises core's typed release error by its message", () => {
		expect(isReleaseFailure(new ApiError('x: release unresolved: unverifiable: y', 422))).toBe(true);
	});

	it('is false for a state violation and for anything that is not an API error', () => {
		expect(isReleaseFailure(new ApiError('cannot update: arrow is updating', 422))).toBe(false);
		expect(isReleaseFailure(new Error('release unresolved: unverifiable'))).toBe(false);
	});
});

describe('isConnectionDrop', () => {
	it('is true for a request that never got an answer', () => {
		expect(isConnectionDrop(new TypeError('Failed to fetch'))).toBe(true);
	});

	it('is true for the proxy reporting the daemon gone', () => {
		expect(isConnectionDrop(new ApiError('bad gateway', 502))).toBe(true);
		expect(isConnectionDrop(new ApiError('timeout', 504))).toBe(true);
	});

	it('is false for an answer from the daemon, whatever it says', () => {
		expect(isConnectionDrop(new ApiError('conflict', 409))).toBe(false);
		expect(isConnectionDrop(new ApiError('boom', 500))).toBe(false);
	});
});
