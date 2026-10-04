import { describe, expect, it, vi } from 'vitest';

import { WS_CLOSE_SENTINEL } from '@/lib/transport/quiver-socket';

import { MAX_SOCKETS_PER_ARROW, SHIM_TAG, createBridge, type WsApi } from './bridge';

type Posted = { type: string; id: string; code?: number; data?: string };

function setup() {
	const sent: Record<string, Posted[]> = { a: [], b: [] };
	const windows = Object.fromEntries(
		['a', 'b'].map((h) => [h, { postMessage: (m: Posted) => sent[h].push(m) } as unknown as Window]),
	);
	const frames: Record<string, (text: string) => void> = {};
	const api = {
		wsOpen: vi.fn(async (host: string, id: string, _p: string, onFrame: (t: string) => void) => {
			frames[`${host}:${id}`] = onFrame;
		}),
		wsSend: vi.fn(async () => {}),
		wsClose: vi.fn(async () => {}),
	} satisfies WsApi;
	const bridge = createBridge({
		api,
		frameWindow: (h) => windows[h],
		hosts: () => Object.keys(windows),
	});
	const from = (host: string, data: unknown) =>
		({ source: windows[host], data: { [SHIM_TAG]: 1, ...(data as object) } }) as unknown as MessageEvent;
	return { api, bridge, sent, frames, from, windows };
}

describe('arrow shell bridge', () => {
	it('opens a socket for the frame the message came from and confirms', async () => {
		const { api, bridge, sent, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));

		expect(api.wsOpen).toHaveBeenCalledWith('a', '1', '/ws', expect.any(Function));
		expect(sent.a).toEqual([expect.objectContaining({ type: 'ws-open', id: '1' })]);
		expect(sent.b).toEqual([]);
	});

	it('ignores messages from windows it does not know or without the tag', async () => {
		const { api, bridge } = setup();
		await bridge.onMessage({ source: {}, data: { [SHIM_TAG]: 1, type: 'ws-open', id: '1', path: '/ws' } } as unknown as MessageEvent);
		await bridge.onMessage({ source: null, data: null } as unknown as MessageEvent);
		await bridge.onMessage({ source: {}, data: { type: 'ws-open' } } as unknown as MessageEvent);
		expect(api.wsOpen).not.toHaveBeenCalled();
	});

	it('drops malformed messages', async () => {
		const { api, bridge, from } = setup();
		for (const bad of [
			{ type: 'ws-open', id: 1, path: '/ws' },
			{ type: 'ws-open', id: '1', path: 5 },
			{ type: 'ws-open', id: 'x'.repeat(40), path: '/ws' },
			{ type: 'nope', id: '1' },
			{ type: 7, id: '1' },
		]) {
			await bridge.onMessage(from('a', bad));
		}
		expect(api.wsOpen).not.toHaveBeenCalled();
	});

	it('forwards frames to the page and turns the close sentinel into ws-close', async () => {
		const { bridge, sent, frames, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));

		frames['a:1']('hello');
		frames['a:1'](`${WS_CLOSE_SENTINEL}\u00001001\u0000bye`);

		expect(sent.a[1]).toEqual(expect.objectContaining({ type: 'ws-message', id: '1', data: 'hello' }));
		expect(sent.a[2]).toEqual(expect.objectContaining({ type: 'ws-close', id: '1' }));
	});

	it('sends only on sockets this frame opened', async () => {
		const { api, bridge, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));

		await bridge.onMessage(from('a', { type: 'ws-send', id: '1', data: 'x' }));
		await bridge.onMessage(from('b', { type: 'ws-send', id: '1', data: 'steal' }));
		await bridge.onMessage(from('a', { type: 'ws-send', id: '1', data: 5 }));

		expect(api.wsSend).toHaveBeenCalledTimes(1);
		expect(api.wsSend).toHaveBeenCalledWith('a', '1', 'x');
	});

	it('closes on request and tells the page', async () => {
		const { api, bridge, sent, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		await bridge.onMessage(from('a', { type: 'ws-close', id: '1' }));

		expect(api.wsClose).toHaveBeenCalledWith('a', '1');
		expect(sent.a[sent.a.length - 1]).toEqual(expect.objectContaining({ type: 'ws-close', id: '1', code: 1000 }));
	});

	it('reports a failed open as error then close 1006', async () => {
		const { api, bridge, sent, from } = setup();
		api.wsOpen.mockRejectedValueOnce(new Error('refused'));
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));

		expect(sent.a.map((m) => m.type)).toEqual(['ws-error', 'ws-close']);
		expect(sent.a[1].code).toBe(1006);
	});

	it('caps the sockets one arrow may hold', async () => {
		const { api, bridge, sent, from } = setup();
		for (let i = 0; i < MAX_SOCKETS_PER_ARROW + 2; i++) {
			await bridge.onMessage(from('a', { type: 'ws-open', id: String(i), path: '/ws' }));
		}
		expect(api.wsOpen).toHaveBeenCalledTimes(MAX_SOCKETS_PER_ARROW);
		expect(sent.a.filter((m) => m.type === 'ws-close')).toHaveLength(2);
	});

	it('closeHost closes everything that arrow held; dispose closes all', async () => {
		const { api, bridge, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		await bridge.onMessage(from('b', { type: 'ws-open', id: '2', path: '/ws' }));

		bridge.closeHost('a');
		expect(api.wsClose).toHaveBeenCalledWith('a', '1');
		bridge.dispose();
		expect(api.wsClose).toHaveBeenCalledWith('b', '2');
	});

	it('reports an abnormal close when the sentinel carries no code', async () => {
		const { bridge, sent, frames, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		frames['a:1'](WS_CLOSE_SENTINEL);

		expect(sent.a[1]).toEqual(expect.objectContaining({ type: 'ws-close', id: '1', code: 1006 }));
	});

	it('ignores a duplicate open for an id already in use', async () => {
		const { api, bridge, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));

		expect(api.wsOpen).toHaveBeenCalledTimes(1);
	});

	it('does not confirm an open that was closed while connecting', async () => {
		const { api, bridge, sent, from } = setup();
		let release = () => {};
		api.wsOpen.mockImplementationOnce(() => new Promise<void>((r) => (release = r)));
		const opening = bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		bridge.closeHost('a');
		release();
		await opening;

		expect(sent.a).toEqual([]);
	});

	it('ignores sends and closes for unknown ids and unrelated types', async () => {
		const { api, bridge, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-send', id: '9', data: 'x' }));
		await bridge.onMessage(from('a', { type: 'ws-close', id: '9' }));
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1' }));

		expect(api.wsSend).not.toHaveBeenCalled();
		expect(api.wsClose).not.toHaveBeenCalled();
		expect(api.wsOpen).not.toHaveBeenCalled();
	});

	it('closeHost on a host with no sockets is a no-op', () => {
		const { api, bridge } = setup();
		bridge.closeHost('a');

		expect(api.wsClose).not.toHaveBeenCalled();
	});
});
