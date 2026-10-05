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

	it.each(['closeHost', 'dispose'] as const)(
		'closes a socket whose open resolves after %s and drops its frames',
		async (how) => {
			const { api, bridge, sent, frames, from } = setup();
			let release = () => {};
			api.wsOpen.mockImplementationOnce(async (host, id, _p, onFrame) => {
				frames[`${host}:${id}`] = onFrame;
				await new Promise<void>((r) => (release = r));
			});
			const opening = bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
			if (how === 'closeHost') bridge.closeHost('a');
			else bridge.dispose();
			release();
			await opening;
			frames['a:1']('late');

			expect(api.wsClose).toHaveBeenCalledTimes(2);
			expect(api.wsClose).toHaveBeenLastCalledWith('a', '1');
			expect(sent.a).toEqual([]);
		},
	);

	it('does not reject when a send fails and reports ws-error', async () => {
		const { api, bridge, sent, from } = setup();
		await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		api.wsSend.mockRejectedValueOnce(new Error('dead'));

		await expect(
			bridge.onMessage(from('a', { type: 'ws-send', id: '1', data: 'x' })),
		).resolves.toBeUndefined();
		expect(sent.a[sent.a.length - 1]).toEqual(expect.objectContaining({ type: 'ws-error', id: '1' }));
	});

	it('still finishes the socket when closing it fails', async () => {
		const { api, bridge, sent, from } = setup();
		for (let i = 0; i < MAX_SOCKETS_PER_ARROW; i++) {
			await bridge.onMessage(from('a', { type: 'ws-open', id: String(i), path: '/ws' }));
		}
		api.wsClose.mockRejectedValueOnce(new Error('gone'));
		await expect(
			bridge.onMessage(from('a', { type: 'ws-close', id: '0' })),
		).resolves.toBeUndefined();
		expect(sent.a[sent.a.length - 1]).toEqual(expect.objectContaining({ type: 'ws-close', id: '0' }));

		await bridge.onMessage(from('a', { type: 'ws-open', id: 'extra', path: '/ws' }));
		expect(api.wsOpen).toHaveBeenCalledTimes(MAX_SOCKETS_PER_ARROW + 1);
	});

	it('swallows close failures during closeHost and late-open cleanup', async () => {
		const { api, bridge, from } = setup();
		let release = () => {};
		api.wsOpen.mockImplementationOnce(async () => {
			await new Promise<void>((r) => (release = r));
		});
		api.wsClose.mockRejectedValue(new Error('gone'));
		const opening = bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
		bridge.closeHost('a');
		release();

		await expect(opening).resolves.toBeUndefined();
		expect(api.wsClose).toHaveBeenCalledTimes(2);
	});

	describe('hello (the frame loaded a new document)', () => {
		it('closes every socket the host held and frees its cap slots', async () => {
			const { api, bridge, from } = setup();
			for (let i = 0; i < MAX_SOCKETS_PER_ARROW; i++) {
				await bridge.onMessage(from('a', { type: 'ws-open', id: String(i), path: '/ws' }));
			}
			await bridge.onMessage(from('a', { type: 'hello' }));

			expect(api.wsClose).toHaveBeenCalledTimes(MAX_SOCKETS_PER_ARROW);
			for (let i = 0; i < MAX_SOCKETS_PER_ARROW; i++) expect(api.wsClose).toHaveBeenCalledWith('a', String(i));
			await bridge.onMessage(from('a', { type: 'ws-open', id: 'fresh', path: '/ws' }));
			expect(api.wsOpen).toHaveBeenCalledTimes(MAX_SOCKETS_PER_ARROW + 1);
		});

		it('is ignored from a window that is not a known frame', async () => {
			const { api, bridge, from } = setup();
			await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
			await bridge.onMessage({ source: {}, data: { [SHIM_TAG]: 1, type: 'hello' } } as unknown as MessageEvent);

			expect(api.wsClose).not.toHaveBeenCalled();
		});

		it('leaves other hosts alone', async () => {
			const { api, bridge, from } = setup();
			await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
			await bridge.onMessage(from('b', { type: 'ws-open', id: '2', path: '/ws' }));
			await bridge.onMessage(from('a', { type: 'hello' }));

			expect(api.wsClose).toHaveBeenCalledTimes(1);
			expect(api.wsClose).toHaveBeenCalledWith('a', '1');
			await bridge.onMessage(from('b', { type: 'ws-send', id: '2', data: 'x' }));
			expect(api.wsSend).toHaveBeenCalledWith('b', '2', 'x');
		});

		it('is still just a hello with extra junk fields', async () => {
			const { api, bridge, from } = setup();
			await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/ws' }));
			await bridge.onMessage(from('a', { type: 'hello', id: 'x'.repeat(99), path: '/ws', data: 5 }));

			expect(api.wsClose).toHaveBeenCalledTimes(1);
			expect(api.wsOpen).toHaveBeenCalledTimes(1);
		});

		it('lets the new document reuse the id of a stale socket', async () => {
			const { api, bridge, sent, from } = setup();
			await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/old' }));
			await bridge.onMessage(from('a', { type: 'hello' }));
			await bridge.onMessage(from('a', { type: 'ws-open', id: '1', path: '/new' }));

			expect(api.wsOpen).toHaveBeenCalledTimes(2);
			expect(api.wsOpen).toHaveBeenLastCalledWith('a', '1', '/new', expect.any(Function));
			expect(sent.a.filter((m) => m.type === 'ws-open')).toHaveLength(2);
		});

		it('does not relay frames from a stale socket into the new document', async () => {
			const { bridge, sent, frames, from } = setup();
			await bridge.onMessage(from('a', { type: 'ws-open', id: 'old-1', path: '/ws' }));
			await bridge.onMessage(from('a', { type: 'hello' }));
			frames['a:old-1']('late');
			frames['a:old-1'](`${WS_CLOSE_SENTINEL}\u00001001\u0000bye`);

			expect(sent.a).toEqual([expect.objectContaining({ type: 'ws-open', id: 'old-1' })]);
		});
	});
});
