import { describe, expect, it, vi } from 'vitest';

import { SOCKET_OPEN } from '@/lib/transport/backend';

import { createSocketHub } from './socket';

const flush = () => new Promise<void>((r) => queueMicrotask(r));

describe('a mock socket', () => {
	it('opens asynchronously, so the manager can attach handlers first', async () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/arrow');
		expect(socket.readyState).not.toBe(SOCKET_OPEN);

		const onopen = vi.fn();
		socket.onopen = onopen;
		await flush();

		expect(socket.readyState).toBe(SOCKET_OPEN);
		expect(onopen).toHaveBeenCalled();
	});

	it('yields no onopen when closed while still connecting', async () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/arrow');
		const onopen = vi.fn();
		socket.onopen = onopen;

		socket.close();
		await flush();

		expect(onopen).not.toHaveBeenCalled();
	});

	it('fires onclose synchronously, once, however often close is called', () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/arrow');
		const onclose = vi.fn();
		socket.onclose = onclose;

		socket.close();
		socket.close();

		expect(onclose).toHaveBeenCalledTimes(1);
	});

	it('drops frames aimed at a socket that has not opened yet', async () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/arrow');
		const onmessage = vi.fn();
		socket.onmessage = onmessage;

		hub.emit('/v0/arrow', { event: 'upserted' });
		expect(onmessage).not.toHaveBeenCalled();

		await flush();
		hub.emit('/v0/arrow', { event: 'upserted' });
		expect(onmessage).toHaveBeenCalledWith({ data: '{"event":"upserted"}' });
	});

	it('has nothing to say on send — both endpoints are transition-only', async () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/runtime');
		await flush();
		expect(() => socket.send('anything')).not.toThrow();
	});
});

describe('the hub', () => {
	it('fans one frame out to every subscriber on that path, and no others', async () => {
		const hub = createSocketHub();
		const a = hub.open('/v0/arrow');
		const b = hub.open('/v0/arrow');
		const other = hub.open('/v0/runtime');
		await flush();

		const seen: string[] = [];
		a.onmessage = (e) => seen.push(`a:${e.data}`);
		b.onmessage = (e) => seen.push(`b:${e.data}`);
		other.onmessage = (e) => seen.push(`other:${e.data}`);

		hub.emit('/v0/arrow', { n: 1 });
		expect(seen).toEqual(['a:{"n":1}', 'b:{"n":1}']);
	});

	it('is a no-op to emit on a path nobody is listening to', () => {
		const hub = createSocketHub();
		expect(() => hub.emit('/v0/nothing', { n: 1 })).not.toThrow();
	});

	it('survives a subscriber that closes itself while the frame is being delivered', async () => {
		const hub = createSocketHub();
		const first = hub.open('/v0/arrow');
		const second = hub.open('/v0/arrow');
		await flush();

		const seen: string[] = [];
		first.onmessage = () => {
			seen.push('first');
			first.close();
		};
		second.onmessage = () => seen.push('second');

		expect(() => hub.emit('/v0/arrow', { n: 1 })).not.toThrow();
		expect(seen).toEqual(['first', 'second']);
	});

	it('forgets a socket once it closes, so the path empties out', async () => {
		const hub = createSocketHub();
		const socket = hub.open('/v0/arrow');
		await flush();
		expect(hub.countFor('/v0/arrow')).toBe(1);

		socket.close();
		expect(hub.countFor('/v0/arrow')).toBe(0);
	});

	it('reports zero for a path that never had a socket', () => {
		expect(createSocketHub().countFor('/v0/never')).toBe(0);
	});

	it('closes everything on closeAll, telling each subscriber', async () => {
		const hub = createSocketHub();
		const a = hub.open('/v0/arrow');
		const b = hub.open('/v0/runtime');
		await flush();

		const closed: string[] = [];
		a.onclose = () => closed.push('arrow');
		b.onclose = () => closed.push('runtime');

		hub.closeAll();

		expect(closed.sort()).toEqual(['arrow', 'runtime']);
		expect(hub.countFor('/v0/arrow')).toBe(0);
		expect(hub.countFor('/v0/runtime')).toBe(0);
	});
});
