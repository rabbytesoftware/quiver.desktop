import { beforeEach, describe, expect, it, vi } from 'vitest';

class FakeSocket {
	static instances: FakeSocket[] = [];
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSED = 3;
	onopen: (() => void) | null = null;
	onmessage: ((e: { data: string }) => void) | null = null;
	onclose: (() => void) | null = null;
	onerror: ((e: unknown) => void) | null = null;
	readyState = FakeSocket.CONNECTING;
	sent: string[] = [];
	constructor(public path: string) {
		FakeSocket.instances.push(this);
	}
	open() {
		this.readyState = FakeSocket.OPEN;
		this.onopen?.();
	}
	send(d: string) {
		this.sent.push(d);
	}
	close() {
		this.readyState = FakeSocket.CLOSED;
	}
}

vi.mock('./quiver-socket', () => ({ QuiverWebSocket: FakeSocket, WS_CLOSE_SENTINEL: '\u0000quiver-ws-close' }));

const { createWSManager, isReconnectSentinel } = await import('./ws-manager');

describe('wsManager', () => {
	beforeEach(() => {
		FakeSocket.instances = [];
		vi.useFakeTimers();
	});

	it('delivers parsed frames to every subscriber', () => {
		const m = createWSManager();
		const a = vi.fn();
		const b = vi.fn();
		m.subscribe('/v0/arrow', a);
		m.subscribe('/v0/arrow', b);
		FakeSocket.instances[0].onmessage?.({ data: '{"event":"upserted"}' });
		expect(a).toHaveBeenCalledWith({ event: 'upserted' });
		expect(b).toHaveBeenCalledWith({ event: 'upserted' });
	});

	it('reuses one socket per endpoint', () => {
		const m = createWSManager();
		m.subscribe('/v0/arrow', vi.fn());
		m.subscribe('/v0/arrow', vi.fn());
		expect(FakeSocket.instances).toHaveLength(1);
	});

	it('keeps the channel open when other subscribers remain', () => {
		const m = createWSManager();
		const offA = m.subscribe('/v0/arrow', vi.fn());
		m.subscribe('/v0/arrow', vi.fn());
		offA();
		m.subscribe('/v0/arrow', vi.fn());
		expect(FakeSocket.instances).toHaveLength(1);
	});

	it('passes an unparseable frame through raw', () => {
		const m = createWSManager();
		const cb = vi.fn();
		m.subscribe('/v0/arrow', cb);
		FakeSocket.instances[0].onmessage?.({ data: 'not json' });
		expect(cb).toHaveBeenCalledWith('not json');
	});

	// Frames missed during an outage cannot be recovered from a DTO merge, so
	// subscribers need to be TOLD the stream broke in order to reseed.
	it('announces a reconnect to subscribers', () => {
		const m = createWSManager();
		const cb = vi.fn();
		m.subscribe('/v0/arrow', cb);
		FakeSocket.instances[0].onclose?.();
		vi.advanceTimersByTime(1000);
		expect(cb).toHaveBeenCalledWith({ reconnected: true });
		expect(isReconnectSentinel({ reconnected: true })).toBe(true);
	});

	it('backs off exponentially across repeated failures', () => {
		const m = createWSManager();
		m.subscribe('/v0/arrow', vi.fn());
		FakeSocket.instances[0].onclose?.();
		vi.advanceTimersByTime(1000);
		expect(FakeSocket.instances).toHaveLength(2);
		FakeSocket.instances[1].onclose?.();
		vi.advanceTimersByTime(1000);
		expect(FakeSocket.instances).toHaveLength(2);
		vi.advanceTimersByTime(1000);
		expect(FakeSocket.instances).toHaveLength(3);
	});

	it('does not resurrect a socket nobody listens to', () => {
		const m = createWSManager();
		const off = m.subscribe('/v0/arrow', vi.fn());
		off();
		FakeSocket.instances[0].onclose?.();
		vi.advanceTimersByTime(5000);
		expect(FakeSocket.instances).toHaveLength(1);
	});

	it('sends only on an open socket', () => {
		const m = createWSManager();
		m.subscribe('/v0/arrow', vi.fn());
		m.send('/v0/arrow', { a: 1 });
		expect(FakeSocket.instances[0].sent).toEqual([]);
		FakeSocket.instances[0].open();
		m.send('/v0/arrow', { a: 1 });
		expect(FakeSocket.instances[0].sent).toEqual(['{"a":1}']);
	});

	// Distinct from "does not resurrect a socket nobody listens to": that test
	// unsubscribes BEFORE onclose ever fires, so the reconnect timer is never
	// even scheduled. Here the timer IS scheduled (a subscriber was still
	// present when the socket closed) and only abandoned during the wait, so
	// this exercises the setTimeout body's own re-check rather than onclose's
	// upfront one.
	it('cleans up a channel abandoned while waiting to reconnect', () => {
		const m = createWSManager();
		const off = m.subscribe('/v0/arrow', vi.fn());
		FakeSocket.instances[0].onclose?.();
		off();
		vi.advanceTimersByTime(1000);
		expect(FakeSocket.instances).toHaveLength(1);
	});

	// A stale unsubscribe (e.g. a caller invoking a cleanup function twice)
	// must only close its own already-dead socket, never the live channel a
	// fresh subscribe has since stood up for the same endpoint.
	it('closes a stale socket without tearing down a fresh resubscription', () => {
		const m = createWSManager();
		const off = m.subscribe('/v0/arrow', vi.fn());
		off();
		m.subscribe('/v0/arrow', vi.fn());
		off();
		expect(FakeSocket.instances).toHaveLength(2);
		expect(FakeSocket.instances[1].readyState).not.toBe(FakeSocket.CLOSED);
		m.subscribe('/v0/arrow', vi.fn());
		expect(FakeSocket.instances).toHaveLength(2);
	});
});
