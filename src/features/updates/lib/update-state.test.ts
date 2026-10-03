import { describe, expect, it } from 'vitest';

import { ApiError } from '@/lib/transport/api';

import { classifyUpdateError, deriveUpdateState, isConnectionDrop } from './update-state';

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

	it('reads a thrown Error as a failure with its message', () => {
		expect(classifyUpdateError(new Error('socket hang up'))).toEqual({ kind: 'failed', message: 'socket hang up' });
	});

	it('reads a thrown non-Error as a failure with its text', () => {
		expect(classifyUpdateError('weird')).toEqual({ kind: 'failed', message: 'weird' });
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
