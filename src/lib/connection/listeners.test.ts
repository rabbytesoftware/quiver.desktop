import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((_event, _handler) => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(() => Promise.resolve({ connections: [], active_id: 'local' })),
}));

const mockSetFromEvent = vi.fn();
vi.mock('./store', () => ({
	useConnectionStore: {
		getState: () => ({ setFromEvent: mockSetFromEvent }),
	},
}));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { setupConnectionListeners } from './listeners';

const invokeMock = invoke as ReturnType<typeof vi.fn>;
const listenMock = listen as ReturnType<typeof vi.fn>;

beforeEach(() => {
	vi.clearAllMocks();
	listenMock.mockImplementation(() => Promise.resolve(() => {}));
	invokeMock.mockResolvedValue({ connections: [], active_id: 'local' });
});

describe('setupConnectionListeners', () => {
	it('registers connection://changed listener', async () => {
		await setupConnectionListeners();
		const channels = listenMock.mock.calls.map((args: unknown[]) => args[0] as string);
		expect(channels).toContain('connection://changed');
	});

	it('calls setFromEvent when connection://changed fires', async () => {
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'connection://changed') {
				handler({
					payload: {
						connections: [
							{
								id: 'c1',
								name: 'Remote',
								kind: 'remote',
								url: 'http://localhost:7070',
								api_version: 'v0',
							},
						],
						active_id: 'c1',
					},
				});
			}
			return Promise.resolve(() => {});
		});

		await setupConnectionListeners();

		expect(mockSetFromEvent).toHaveBeenCalledWith(
			[{ id: 'c1', name: 'Remote', kind: 'remote', url: 'http://localhost:7070', api_version: 'v0' }],
			'c1'
		);
	});

	// `connection://changed` fires on add/remove/rename only, and
	// `switch_connection` emits nothing at all — so a session where nobody
	// touches a host used to leave this store at its empty initial value for its
	// whole life. Invisible until something rendered the list.
	it('seeds the store from the current connection list, without waiting for an event', async () => {
		const connections = [{ id: 'local', name: 'Local', kind: 'local' as const, api_version: 'v0' }];
		invokeMock.mockResolvedValue({ connections, active_id: 'local' });

		await setupConnectionListeners();

		expect(invokeMock).toHaveBeenCalledWith('get_connections');
		expect(mockSetFromEvent).toHaveBeenCalledWith(connections, 'local');
	});

	// The subscription is what recovers from this, so a failed seed must not
	// take the whole listener down with it.
	it('survives a failed seed with the subscription still registered', async () => {
		invokeMock.mockRejectedValue(new Error('ipc gone'));
		vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(setupConnectionListeners()).resolves.toBeUndefined();

		const channels = listenMock.mock.calls.map((args: unknown[]) => args[0] as string);
		expect(channels).toContain('connection://changed');
	});
});
