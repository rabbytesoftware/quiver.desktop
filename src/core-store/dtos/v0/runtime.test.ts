import { describe, it, expect } from 'vitest';

import { toRuntimeUpdate } from './runtime';

describe('toRuntimeUpdate', () => {
	it('maps namespace and state', () => {
		const result = toRuntimeUpdate({ namespace: 'ns@v1', state: 'running' });
		expect(result.namespace).toBe('ns@v1');
		expect(result.state).toBe('running');
	});

	it('defaults active_run to null when absent', () => {
		const result = toRuntimeUpdate({ namespace: 'ns@v1', state: 'ready' });
		expect(result.active_run).toBeNull();
	});

	it('preserves provided active_run', () => {
		const run = { method: '_install', steps: [], variables: {}, pid: undefined };
		const result = toRuntimeUpdate({ namespace: 'ns@v1', state: 'installing', active_run: run });
		expect(result.active_run).toEqual(run);
	});

	it('defaults last_outcome to null when last_return absent', () => {
		const result = toRuntimeUpdate({ namespace: 'ns@v1', state: 'ready' });
		expect(result.last_outcome).toBeNull();
	});

	it('maps last_return wire field to last_outcome domain field', () => {
		const lastReturn = { method: '_install', outcome: 'success' as const, variables: {}, steps: [] };
		const result = toRuntimeUpdate({ namespace: 'ns@v1', state: 'ready', last_return: lastReturn });
		expect(result.last_outcome).toEqual(lastReturn);
	});
});
