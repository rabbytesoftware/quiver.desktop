import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((_event, _handler) => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('../store/arrows', () => ({
	useArrowStore: {
		getState: () => ({
			resetArrows: vi.fn(),
			upsertArrow: vi.fn(),
			removeArrow: vi.fn(),
			applyRuntimeUpdate: vi.fn(),
		}),
	},
}));

vi.mock('../store/status', () => ({
	useStatusStore: {
		getState: () => ({ setStatus: vi.fn() }),
	},
}));

vi.mock('@/lib/connection/store', () => ({
	useConnectionStore: {
		getState: () => ({ setFromEvent: vi.fn() }),
	},
}));

import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { setupListeners } from './index';

beforeEach(() => vi.clearAllMocks());

describe('setupListeners', () => {
	it('registers core://status, arrow://event, runtime://update, and connection://changed listeners', async () => {
		await setupListeners();
		const listenMock = listen as ReturnType<typeof vi.fn>;
		const channels = listenMock.mock.calls.map((args: unknown[]) => args[0] as string);
		expect(channels).toContain('core://status');
		expect(channels).toContain('arrow://event');
		expect(channels).toContain('runtime://update');
		expect(channels).toContain('connection://changed');
		expect(channels).not.toContain('arrow://hydrate');
	});

	it('calls get_arrows and get_connections on core://status ready', async () => {
		const listenMock = listen as ReturnType<typeof vi.fn>;
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'core://status') {
				handler({ payload: { status: 'ready' } });
			}
			return Promise.resolve(() => {});
		});
		await setupListeners();
		expect(invoke).toHaveBeenCalledWith('get_arrows');
		expect(invoke).toHaveBeenCalledWith('get_connections');
	});
});
