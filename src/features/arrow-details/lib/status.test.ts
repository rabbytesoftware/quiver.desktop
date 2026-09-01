import { describe, expect, it } from 'vitest';

import type { ArrowState, LastReturnDetail } from '@/domain/arrow';

import { computeStatus, problemMessage } from './status';

describe('computeStatus', () => {
	it('is "discovered"/idle whenever user_installed is false, regardless of state', () => {
		expect(computeStatus({ state: 'absent', user_installed: false })).toEqual({
			labelKey: 'arrow.state.discovered',
			iconKind: 'idle',
		});
		expect(computeStatus({ state: 'running', user_installed: false })).toEqual({
			labelKey: 'arrow.state.discovered',
			iconKind: 'idle',
		});
	});

	const CASES: Array<[ArrowState, string, string]> = [
		['absent', 'arrow.state.absent', 'idle'],
		['installing', 'arrow.state.installing', 'busy'],
		['ready', 'arrow.state.ready', 'ready'],
		['outdated', 'arrow.state.outdated', 'up'],
		['updating', 'arrow.state.updating', 'busy'],
		['running', 'arrow.state.running', 'active'],
		['stopping', 'arrow.state.stopping', 'busy'],
		['draining', 'arrow.state.draining', 'busy'],
		['detached', 'arrow.state.detached', 'problem'],
		['uninstalling', 'arrow.state.uninstalling', 'busy'],
		['removed', 'arrow.state.removed', 'archived'],
	];

	it.each(CASES)('maps %s to %s / %s when in the library', (state, labelKey, iconKind) => {
		expect(computeStatus({ state, user_installed: true })).toEqual({ labelKey, iconKind });
	});

	it('falls back to absent/idle for an unrecognized state rather than throwing', () => {
		expect(computeStatus({ state: 'nonsense' as unknown as ArrowState, user_installed: true })).toEqual({
			labelKey: 'arrow.state.absent',
			iconKind: 'idle',
		});
	});
});

describe('problemMessage', () => {
	it('flags detached regardless of last_return', () => {
		expect(problemMessage({ state: 'detached', last_return: null })).toEqual({ reason: 'detached' });
	});

	it('flags a failed last_return with the failed step’s own error text', () => {
		const lastReturn: LastReturnDetail = {
			method: 'install',
			outcome: 'failed',
			variables: {},
			steps: [
				{ index: 0, title: 'Resolve manifest', status: 'completed', type: 'run' },
				{ index: 1, title: 'Verify checksum', status: 'failed', type: 'run', error: 'checksum mismatch' },
			],
		};
		expect(problemMessage({ state: 'ready', last_return: lastReturn })).toEqual({
			reason: 'failed',
			detail: 'checksum mismatch',
		});
	});

	it('flags a failed last_return with no detail when no step carries an error', () => {
		const lastReturn: LastReturnDetail = {
			method: 'install',
			outcome: 'failed',
			variables: {},
			steps: [{ index: 0, title: 'Resolve manifest', status: 'failed', type: 'run' }],
		};
		expect(problemMessage({ state: 'ready', last_return: lastReturn })).toEqual({
			reason: 'failed',
			detail: undefined,
		});
	});

	it('returns null for a successful last_return', () => {
		const lastReturn: LastReturnDetail = { method: 'install', outcome: 'success', variables: {}, steps: [] };
		expect(problemMessage({ state: 'ready', last_return: lastReturn })).toBeNull();
	});

	it('returns null when there is no last_return at all', () => {
		expect(problemMessage({ state: 'ready', last_return: null })).toBeNull();
	});
});
