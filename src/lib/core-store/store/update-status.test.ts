import { describe, it, expect, beforeEach } from 'vitest';

import { useCoreUpdateStatusStore } from './update-status';

beforeEach(() => {
	useCoreUpdateStatusStore.setState({ status: { outdated: false } });
});

describe('useCoreUpdateStatusStore', () => {
	it('has correct initial status', () => {
		expect(useCoreUpdateStatusStore.getState().status).toEqual({ outdated: false });
	});

	it('setStatus updates to outdated with a recommended ref', () => {
		useCoreUpdateStatusStore.getState().setStatus({ outdated: true, recommended_ref: 'stable-26.0.0' });
		expect(useCoreUpdateStatusStore.getState().status).toEqual({
			outdated: true,
			recommended_ref: 'stable-26.0.0',
		});
	});

	it('setStatus can clear back to not outdated', () => {
		useCoreUpdateStatusStore.getState().setStatus({ outdated: true, recommended_ref: 'stable-26.0.0' });
		useCoreUpdateStatusStore.getState().setStatus({ outdated: false });
		expect(useCoreUpdateStatusStore.getState().status).toEqual({ outdated: false });
	});
});
