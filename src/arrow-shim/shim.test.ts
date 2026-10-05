import { describe, expect, it, vi } from 'vitest';

import shimSource from './shim.js?raw';

type Posted = { type: string; id: string; path?: string; data?: string };

/** Runs the shim against a fake window whose parent records postMessage. */
function load(href = 'arrow-app://abc/index.html') {
	const posted: Posted[] = [];
	const parent = { postMessage: (m: Posted) => posted.push(m) };
	const listeners: Record<string, (e: unknown) => void> = {};
	const fakeWindow = {
		parent,
		location: { href },
		addEventListener: (type: string, fn: (e: unknown) => void) => {
			listeners[type] = fn;
		},
	} as Record<string, unknown>;
	new Function('window', shimSource)(fakeWindow);
	const WS = fakeWindow.WebSocket as unknown as typeof WebSocket;
	const opens = () => posted.filter((p) => p.type === 'ws-open');
	const deliver = (m: object, source: unknown = parent) =>
		listeners.message({ source, data: { __arrowShim: 1, ...m } });
	return { WS, posted, opens, deliver, parent };
}

describe('arrow websocket shim', () => {
	it('opens by path and ignores the host, so an arrow can only reach itself', () => {
		const { WS, opens } = load();
		const ws = new WS('ws://evil.example/ws?username=bob');
		expect(ws.readyState).toBe(0);
		expect(opens()).toEqual([expect.objectContaining({ type: 'ws-open', path: '/ws?username=bob' })]);
	});

	it('resolves relative urls against the page', () => {
		const { WS, opens } = load('arrow-app://abc/chat/index.html');
		new WS('/socket');
		expect(opens()[0].path).toBe('/socket');
	});

	it('rejects non websocket schemes', () => {
		const { WS } = load();
		expect(() => new WS('ftp://x/y')).toThrow();
	});

	it('fires open, message and close from the shell', () => {
		const { WS, opens, deliver } = load();
		const ws = new WS('ws://h/ws');
		const events: string[] = [];
		ws.onopen = () => events.push('open');
		ws.onmessage = (e: MessageEvent) => events.push(`msg:${e.data}`);
		ws.addEventListener('close', (e: CloseEvent) => events.push(`close:${e.code}:${e.reason}`));

		deliver({ type: 'ws-open', id: opens()[0].id });
		expect(ws.readyState).toBe(1);
		deliver({ type: 'ws-message', id: opens()[0].id, data: 'hello' });
		deliver({ type: 'ws-close', id: opens()[0].id, code: 1001, reason: 'bye' });

		expect(events).toEqual(['open', 'msg:hello', 'close:1001:bye']);
		expect(ws.readyState).toBe(3);
	});

	it('sends text only and refuses to send while connecting', () => {
		const { WS, opens, posted, deliver } = load();
		const err = vi.spyOn(console, 'error').mockImplementation(() => {});
		const ws = new WS('ws://h/ws');
		expect(() => ws.send('early')).toThrow();

		deliver({ type: 'ws-open', id: opens()[0].id });
		ws.send('hi');
		ws.send(new ArrayBuffer(2));
		expect(posted.filter((p) => p.type === 'ws-send')).toEqual([expect.objectContaining({ data: 'hi' })]);
		expect(err).toHaveBeenCalled();
	});

	it('close() asks the shell and finishes when it confirms', () => {
		const { WS, opens, posted, deliver } = load();
		const ws = new WS('ws://h/ws');
		deliver({ type: 'ws-open', id: opens()[0].id });

		ws.close();
		expect(ws.readyState).toBe(2);
		expect(posted[posted.length - 1]).toEqual(expect.objectContaining({ type: 'ws-close' }));
		ws.close();
		expect(posted.filter((p) => p.type === 'ws-close')).toHaveLength(1);

		deliver({ type: 'ws-close', id: opens()[0].id, code: 1000 });
		expect(ws.readyState).toBe(3);
	});

	it('ignores messages that do not come from the parent window', () => {
		const { WS, opens, deliver } = load();
		const ws = new WS('ws://h/ws');
		deliver({ type: 'ws-open', id: opens()[0].id }, { not: 'parent' });
		expect(ws.readyState).toBe(0);
	});

	it('ignores unknown ids and untagged messages', () => {
		const { WS, opens, deliver } = load();
		const ws = new WS('ws://h/ws');
		deliver({ type: 'ws-open', id: 'nope' });
		expect(ws.readyState).toBe(0);
		expect(opens()).toHaveLength(1);
	});

	it('error then close surfaces both events', () => {
		const { WS, opens, deliver } = load();
		const ws = new WS('ws://h/ws');
		const events: string[] = [];
		ws.onerror = () => events.push('error');
		ws.onclose = (e: CloseEvent) => events.push(`close:${e.code}`);
		deliver({ type: 'ws-error', id: opens()[0].id });
		deliver({ type: 'ws-close', id: opens()[0].id, code: 1006 });
		expect(events).toEqual(['error', 'close:1006']);
	});

	it('exposes the standard state constants', () => {
		const { WS } = load();
		expect([WS.CONNECTING, WS.OPEN, WS.CLOSING, WS.CLOSED]).toEqual([0, 1, 2, 3]);
	});

	it('posts hello exactly once, first, before any socket message', () => {
		const { WS, posted } = load();
		new WS('ws://h/a');
		new WS('ws://h/b');
		expect(posted[0]).toEqual({ type: 'hello', __arrowShim: 1 });
		expect(posted.filter((p) => p.type === 'hello')).toHaveLength(1);
	});

	it('gives each socket a distinct id sharing one per-document nonce', () => {
		const { WS, opens } = load();
		new WS('ws://h/a');
		new WS('ws://h/b');
		const [a, b] = opens().map((p) => p.id);
		expect(a).not.toBe(b);
		for (const id of [a, b]) {
			expect(id).toMatch(/^[a-z0-9]+-\d+$/);
			expect(id.length).toBeLessThanOrEqual(32);
		}
		expect(a.split('-')[0]).toBe(b.split('-')[0]);
	});

	it('derives the nonce from Math.random so each document differs', () => {
		const random = vi.spyOn(Math, 'random');
		const nonceOf = (value: number) => {
			random.mockReturnValue(value);
			const { WS, opens } = load();
			new WS('ws://h/a');
			return opens()[0].id.split('-')[0];
		};
		const first = nonceOf(0.123456789);
		const second = nonceOf(0.987654321);
		random.mockRestore();
		expect(first).not.toBe(second);
	});

	it('still produces a non-empty nonce when Math.random returns 0', () => {
		const random = vi.spyOn(Math, 'random').mockReturnValue(0);
		const { WS, opens } = load();
		new WS('ws://h/a');
		random.mockRestore();
		expect(opens()[0].id).toMatch(/^[a-z0-9]+-1$/);
	});
});
