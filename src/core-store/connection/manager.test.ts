import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { useConnectionStore, loadConnections } from './manager';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

beforeEach(() => {
	mockInvoke.mockResolvedValue([] as never);
	useConnectionStore.setState({ connections: [{ id: 'local', name: 'Local', kind: 'local', api_version: 'v0' }], activeId: 'local' });
});

describe('useConnectionStore', () => {
	it('starts with local connection active', () => {
		const { connections, activeId } = useConnectionStore.getState();
		expect(activeId).toBe('local');
		expect(connections[0].id).toBe('local');
	});
});

describe('loadConnections', () => {
	it('calls list_connections and updates store', async () => {
		const conns = [
			{ id: 'r1', name: 'Remote', kind: 'remote', url: 'tcp://10.0.0.1:40257', api_version: 'v0' },
		];
		mockInvoke.mockResolvedValueOnce(conns as never);
		await loadConnections();
		expect(invoke).toHaveBeenCalledWith('list_connections');
		expect(useConnectionStore.getState().connections).toEqual(conns);
	});

	it('updates store with empty list', async () => {
		await loadConnections();
		expect(useConnectionStore.getState().connections).toEqual([]);
	});
});
