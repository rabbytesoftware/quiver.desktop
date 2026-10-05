import { describe, expect, it, vi } from 'vitest';

import type { SocketLike } from '@/lib/transport/backend';

import type { LogFrame } from './frames';
import { createLogStream, streamPath, type LogStreamState } from './log-stream';

class FakeSocket implements SocketLike {
	readyState = 0;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onclose: (() => void) | null = null;
	onerror: ((event: unknown) => void) | null = null;
	closed = false;
	constructor(readonly path: string) {}
	send(): void {}
	close(): void {
		this.closed = true;
	}
	open(): void {
		this.onopen?.();
	}
	deliver(text: string): void {
		this.onmessage?.({ data: text });
	}
	drop(): void {
		this.onclose?.();
	}
}

function harness(cursor: () => number | null = () => null) {
	const sockets: FakeSocket[] = [];
	const frames: LogFrame[] = [];
	const states: LogStreamState[] = [];
	const pending: { fn: () => void; ms: number; handle: number; live: boolean }[] = [];
	let nextHandle = 1;
	const stream = createLogStream({
		open: (path) => {
			const socket = new FakeSocket(path);
			sockets.push(socket);
			return socket;
		},
		cursor,
		onFrames: (f) => frames.push(...f),
		onState: (s) => states.push(s),
		timers: {
			set: (fn, ms) => {
				const handle = nextHandle++;
				pending.push({ fn, ms, handle, live: true });
				return handle;
			},
			clear: (handle) => {
				const t = pending.find((p) => p.handle === handle);
				if (t) t.live = false;
			},
		},
	});
	const fire = (): number => {
		const t = pending.find((p) => p.live);
		if (!t) throw new Error('no timer pending');
		t.live = false;
		t.fn();
		return t.ms;
	};
	return { stream, sockets, frames, states, pending, fire };
}

describe('streamPath', () => {
	it('asks for debug, and the daemon decides how much to replay', () => {
		expect(streamPath(null)).toBe('/v0/console/logs?level=debug');
	});

	it('resumes after the cursor', () => {
		expect(streamPath(412)).toBe('/v0/console/logs?level=debug&since=412');
		expect(streamPath(0)).toContain('since=0');
	});
});

describe('the log stream', () => {
	it('connects, reports live once open, and hands frames on decoded', () => {
		const h = harness();
		h.stream.start();
		expect(h.states).toEqual(['connecting']);
		h.sockets[0].open();
		expect(h.states).toEqual(['connecting', 'live']);

		h.sockets[0].deliver('{"type":"ready","seq":3}');
		h.sockets[0].deliver('not json');
		expect(h.frames).toEqual([
			{ type: 'ready', seq: 3, reset: false },
			{ type: 'raw', text: 'not json' },
		]);
	});

	it('does nothing when started twice', () => {
		const h = harness();
		h.stream.start();
		h.stream.start();
		expect(h.sockets).toHaveLength(1);
		expect(h.stream.running()).toBe(true);
	});

	it('reconnects with backoff after a drop, resuming from the cursor', () => {
		let cursor: number | null = null;
		const h = harness(() => cursor);
		h.stream.start();
		h.sockets[0].open();
		cursor = 41;
		h.sockets[0].drop();
		expect(h.states[h.states.length - 1]).toBe('reconnecting');

		expect(h.fire()).toBe(1000);
		expect(h.sockets).toHaveLength(2);
		expect(h.sockets[1].path).toContain('since=41');
	});

	it('doubles the wait to a ceiling, and a good connection resets it', () => {
		const h = harness();
		h.stream.start();
		const waits: number[] = [];
		for (let i = 0; i < 8; i++) {
			h.sockets[h.sockets.length - 1].drop();
			waits.push(h.fire());
		}
		expect(waits).toEqual([1000, 2000, 4000, 8000, 16_000, 30_000, 30_000, 30_000]);

		h.sockets[h.sockets.length - 1].open();
		h.sockets[h.sockets.length - 1].drop();
		expect(h.fire()).toBe(1000);
	});

	it('says reconnecting, not connecting, on later attempts', () => {
		const h = harness();
		h.stream.start();
		h.sockets[0].drop();
		h.fire();
		expect(h.states).toEqual(['connecting', 'reconnecting', 'reconnecting']);
	});

	it('stop closes the socket, cancels a pending reconnect, and goes idle', () => {
		const h = harness();
		h.stream.start();
		h.sockets[0].drop();
		h.stream.stop();
		expect(h.pending.every((p) => !p.live)).toBe(true);
		expect(h.states[h.states.length - 1]).toBe('idle');
		expect(h.stream.running()).toBe(false);

		h.stream.start();
		h.stream.stop();
		expect(h.sockets[1].closed).toBe(true);
	});

	it('ignores everything a socket does after it was replaced or stopped', () => {
		const h = harness();
		h.stream.start();
		const old = h.sockets[0];
		h.stream.stop();
		old.onopen?.();
		old.onmessage?.({ data: '{"type":"ready","seq":1}' });
		old.onclose?.();
		expect(h.frames).toEqual([]);
		expect(h.pending.filter((p) => p.live)).toEqual([]);
	});

	it('a socket that closes on stop does not schedule a reconnect', () => {
		const h = harness();
		h.stream.start();
		const socket = h.sockets[0];
		socket.close = () => socket.drop();
		h.stream.stop();
		expect(h.pending).toEqual([]);
	});

	it('ignores a socket error: the close that follows is handled', () => {
		const h = harness();
		h.stream.start();
		expect(() => h.sockets[0].onerror?.(new Error('boom'))).not.toThrow();
		expect(h.sockets).toHaveLength(1);
	});

	it('uses real timers when none are injected', () => {
		vi.useFakeTimers();
		try {
			const sockets: FakeSocket[] = [];
			const stream = createLogStream({
				open: (p) => {
					const s = new FakeSocket(p);
					sockets.push(s);
					return s;
				},
				cursor: () => null,
				onFrames: () => {},
				onState: () => {},
			});
			stream.start();
			sockets[0].drop();
			vi.advanceTimersByTime(1000);
			expect(sockets).toHaveLength(2);
			stream.stop();
			expect(vi.getTimerCount()).toBe(0);
		} finally {
			vi.useRealTimers();
		}
	});
});
