import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((_event, _handler) => Promise.resolve(() => {})),
}));

const mockSetFromEvent = vi.fn();
vi.mock('./store', () => ({
	useConnectionStore: {
		getState: () => ({ setFromEvent: mockSetFromEvent }),
	},
}));

import { listen } from '@tauri-apps/api/event';

import { setupConnectionListeners } from './listeners';

beforeEach(() => vi.clearAllMocks());

describe('setupConnectionListeners', () => {
	it('registers connection://changed listener', async () => {
		await setupConnectionListeners();
		const listenMock = listen as ReturnType<typeof vi.fn>;
		const channels = listenMock.mock.calls.map((args: unknown[]) => args[0] as string);
		expect(channels).toContain('connection://changed');
	});

	it('calls setFromEvent when connection://changed fires', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
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
});
