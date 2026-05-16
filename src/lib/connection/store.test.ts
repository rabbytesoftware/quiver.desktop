import { describe, it, expect, beforeEach } from 'vitest';

import { useConnectionStore } from './store';

beforeEach(() => {
	useConnectionStore.setState({ connections: [], activeId: 'local' });
});

describe('useConnectionStore', () => {
	it('has correct initial state', () => {
		const state = useConnectionStore.getState();
		expect(state.connections).toEqual([]);
		expect(state.activeId).toBe('local');
	});

	it('setFromEvent updates connections and activeId', () => {
		const connections = [
			{ id: 'conn-1', name: 'Remote', kind: 'remote' as const, url: 'http://localhost:7070', api_version: 'v0' },
		];
		useConnectionStore.getState().setFromEvent(connections, 'conn-1');
		const state = useConnectionStore.getState();
		expect(state.connections).toEqual(connections);
		expect(state.activeId).toBe('conn-1');
	});

	it('setFromEvent with empty connections clears the list', () => {
		useConnectionStore.setState({
			connections: [{ id: 'old', name: 'Old', kind: 'local', api_version: 'v0' }],
			activeId: 'old',
		});
		useConnectionStore.getState().setFromEvent([], 'local');
		const state = useConnectionStore.getState();
		expect(state.connections).toEqual([]);
		expect(state.activeId).toBe('local');
	});
});
