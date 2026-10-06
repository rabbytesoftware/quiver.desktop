import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/core', () => ({
	invoke,
	Channel: class {
		onmessage: ((text: string) => void) | null = null;
	},
}));

import { tauriApi } from './api';

describe('tauriApi', () => {
	beforeEach(() => {
		invoke.mockReset();
		invoke.mockResolvedValue(undefined);
	});

	it('asks for the host of a namespace', async () => {
		invoke.mockResolvedValueOnce('abc.arrow');
		await expect(tauriApi.host('ns')).resolves.toBe('abc.arrow');
		expect(invoke).toHaveBeenCalledWith('arrow_app_host', { namespace: 'ns' });
	});

	it('opens a socket with a channel wired to the frame callback', async () => {
		const onFrame = vi.fn();
		await tauriApi.wsOpen('h', 'c1', '/ws', onFrame);

		expect(invoke).toHaveBeenCalledWith('arrow_ws_open', {
			host: 'h',
			connId: 'c1',
			path: '/ws',
			onMessage: expect.objectContaining({ onmessage: onFrame }),
		});
	});

	it('sends and closes by connection id', async () => {
		await tauriApi.wsSend('h', 'c1', 'x');
		await tauriApi.wsClose('h', 'c1');

		expect(invoke).toHaveBeenCalledWith('arrow_ws_send', { host: 'h', connId: 'c1', data: 'x' });
		expect(invoke).toHaveBeenCalledWith('arrow_ws_close', { host: 'h', connId: 'c1' });
	});
});
