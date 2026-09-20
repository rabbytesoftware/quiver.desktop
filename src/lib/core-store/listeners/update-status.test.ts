import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(),
}));

import { listen } from '@tauri-apps/api/event';

import { listenForCoreUpdateStatus } from './update-status';

const listenMock = listen as ReturnType<typeof vi.fn>;

beforeEach(() => {
	vi.clearAllMocks();
	listenMock.mockImplementation(() => Promise.resolve(() => {}));
});

describe('listenForCoreUpdateStatus', () => {
	it('registers a core://update_status listener', async () => {
		await listenForCoreUpdateStatus(vi.fn());
		expect(listenMock).toHaveBeenCalledWith('core://update_status', expect.any(Function));
	});

	it('invokes the callback with the emitted payload', async () => {
		const cb = vi.fn();
		listenMock.mockImplementation((event: string, handler: (e: unknown) => void) => {
			if (event === 'core://update_status') {
				handler({ payload: { status: { outdated: true, recommended_ref: 'stable-26.0.0' } } });
			}
			return Promise.resolve(() => {});
		});

		const unlisten = await listenForCoreUpdateStatus(cb);

		expect(cb).toHaveBeenCalledWith({ outdated: true, recommended_ref: 'stable-26.0.0' });
		expect(typeof unlisten).toBe('function');
	});
});
