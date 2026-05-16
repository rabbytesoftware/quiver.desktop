import { describe, it, expect, beforeEach } from 'vitest';

import { useStatusStore } from './status';

beforeEach(() => {
	useStatusStore.setState({ status: 'starting' });
});

describe('useStatusStore', () => {
	it('has correct initial status', () => {
		expect(useStatusStore.getState().status).toBe('starting');
	});

	it('setStatus updates the status to ready', () => {
		useStatusStore.getState().setStatus('ready');
		expect(useStatusStore.getState().status).toBe('ready');
	});

	it('setStatus updates the status to disconnected', () => {
		useStatusStore.getState().setStatus('disconnected');
		expect(useStatusStore.getState().status).toBe('disconnected');
	});
});
