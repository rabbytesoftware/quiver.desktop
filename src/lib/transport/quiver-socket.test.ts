import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
	invoke: (...args: unknown[]) => invoke(...args),
	Channel: class {
		onmessage: ((m: string) => void) | null = null;
	},
}));

const { QuiverWebSocket, WS_CLOSE_SENTINEL } = await import('./quiver-socket');

describe('QuiverWebSocket', () => {
	beforeEach(() => {
		invoke.mockReset();
		invoke.mockResolvedValue(undefined);
	});

	it('opens and fires onopen once ws_open resolves', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		const opened = vi.fn();
		ws.onopen = opened;
		await vi.waitFor(() => expect(opened).toHaveBeenCalled());
		expect(ws.readyState).toBe(QuiverWebSocket.OPEN);
		expect(invoke).toHaveBeenCalledWith('ws_open', expect.objectContaining({ connId: expect.any(String) }));
	});

	it('is the exact sentinel bridge.rs sends, independent of the import', () => {
		expect(WS_CLOSE_SENTINEL).toBe('\u0000quiver-ws-close');
	});

	it('surfaces a frame as a MessageEvent-shaped object', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		const seen: string[] = [];
		ws.onmessage = (e) => seen.push(e.data);
		const channel = invoke.mock.calls[0][1].onMessage as { onmessage: (m: string) => void };
		channel.onmessage('{"event":"upserted"}');
		expect(seen).toEqual(['{"event":"upserted"}']);
	});

	it('treats the sentinel as a close, not a message', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		const closed = vi.fn();
		const message = vi.fn();
		ws.onclose = closed;
		ws.onmessage = message;
		const channel = invoke.mock.calls[0][1].onMessage as { onmessage: (m: string) => void };
		channel.onmessage(WS_CLOSE_SENTINEL);
		expect(closed).toHaveBeenCalled();
		expect(message).not.toHaveBeenCalled();
		expect(ws.readyState).toBe(QuiverWebSocket.CLOSED);
	});

	it('defers ws_close when closed while still connecting', async () => {
		let resolveOpen: () => void = () => {};
		invoke.mockImplementation((cmd: string) => {
			if (cmd === 'ws_open') return new Promise<void>((r) => (resolveOpen = r));
			return Promise.resolve();
		});
		const ws = new QuiverWebSocket('/v0/arrow');
		ws.close();
		expect(invoke).not.toHaveBeenCalledWith('ws_close', expect.anything());
		resolveOpen();
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith('ws_close', expect.objectContaining({ connId: expect.any(String) }))
		);
	});

	it('reports a failed open as error then close', async () => {
		invoke.mockImplementation((cmd: string) =>
			cmd === 'ws_open' ? Promise.reject(new Error('no socket')) : Promise.resolve()
		);
		const ws = new QuiverWebSocket('/v0/arrow');
		const onerror = vi.fn();
		const onclose = vi.fn();
		ws.onerror = onerror;
		ws.onclose = onclose;
		await vi.waitFor(() => expect(onerror).toHaveBeenCalled());
		expect(onclose).toHaveBeenCalled();
	});

	it('sends through ws_send', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		ws.send('{"hello":1}');
		expect(invoke).toHaveBeenCalledWith(
			'ws_send',
			expect.objectContaining({ connId: expect.any(String), data: '{"hello":1}' })
		);
	});

	it('closes eagerly once already open, not deferred', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		await vi.waitFor(() => expect(ws.readyState).toBe(QuiverWebSocket.OPEN));
		invoke.mockClear();
		ws.close();
		expect(invoke).toHaveBeenCalledWith('ws_close', expect.objectContaining({ connId: expect.any(String) }));
	});

	it('ignores a second sentinel once already closed', async () => {
		const ws = new QuiverWebSocket('/v0/arrow');
		const closed = vi.fn();
		ws.onclose = closed;
		const channel = invoke.mock.calls[0][1].onMessage as { onmessage: (m: string) => void };
		channel.onmessage(WS_CLOSE_SENTINEL);
		channel.onmessage(WS_CLOSE_SENTINEL);
		expect(closed).toHaveBeenCalledTimes(1);
	});

	it('does not double-report once closed before ws_open rejects', async () => {
		const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
		let rejectOpen: (err: Error) => void = () => {};
		invoke.mockImplementation((cmd: string) => {
			if (cmd === 'ws_open') return new Promise<void>((_resolve, reject) => (rejectOpen = reject));
			return Promise.resolve();
		});
		const ws = new QuiverWebSocket('/v0/arrow');
		const onerror = vi.fn();
		const onclose = vi.fn();
		ws.onerror = onerror;
		ws.onclose = onclose;
		ws.close();
		expect(onclose).toHaveBeenCalledTimes(1);
		rejectOpen(new Error('no socket'));
		await flush();
		expect(onerror).not.toHaveBeenCalled();
		expect(onclose).toHaveBeenCalledTimes(1);
	});
});
