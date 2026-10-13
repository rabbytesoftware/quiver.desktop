import type { SocketCloseInfo, SocketLike } from '@/lib/transport/backend';

import type { Emitter } from './world/types';

const CONNECTING = 0;
const OPEN = 1;
const CLOSED = 3;

export class MockWebSocket implements SocketLike {
	readyState = CONNECTING;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onclose: ((info?: SocketCloseInfo) => void) | null = null;
	onerror: ((event: unknown) => void) | null = null;

	constructor(
		readonly path: string,
		private readonly detach: (socket: MockWebSocket) => void
	) {
		queueMicrotask(() => {
			if (this.readyState === CLOSED) return;
			this.readyState = OPEN;
			this.onopen?.();
		});
	}

	deliver(text: string): void {
		if (this.readyState !== OPEN) return;
		this.onmessage?.({ data: text });
	}

	send(_data: string): void {}

	close(info?: SocketCloseInfo): void {
		if (this.readyState === CLOSED) return;
		this.readyState = CLOSED;
		this.detach(this);
		this.onclose?.(info);
	}
}

export interface SocketHub extends Emitter {
	open(path: string): MockWebSocket;
	closeAll(): void;
	countFor(path: string): number;
}

export function createSocketHub(): SocketHub {
	const byPath = new Map<string, Set<MockWebSocket>>();

	function detach(socket: MockWebSocket): void {
		const set = byPath.get(socket.path);
		if (!set) return;
		set.delete(socket);
		if (set.size === 0) byPath.delete(socket.path);
	}

	return {
		open(requested) {
			// Filters ride the query string; the stream is identified by its path.
			const path = requested.split('?')[0];
			const socket = new MockWebSocket(path, detach);
			const set = byPath.get(path) ?? new Set<MockWebSocket>();
			set.add(socket);
			byPath.set(path, set);
			return socket;
		},

		emit(endpoint, frame) {
			const set = byPath.get(endpoint);
			if (!set || set.size === 0) return;
			const text = JSON.stringify(frame);
			[...set].forEach((socket) => socket.deliver(text));
		},

		close(endpoint, code, reason) {
			for (const socket of [...(byPath.get(endpoint) ?? [])]) socket.close({ code, reason });
		},

		closeAll() {
			for (const set of byPath.values()) {
				for (const socket of [...set]) socket.close();
			}
			byPath.clear();
		},

		countFor(path) {
			return byPath.get(path)?.size ?? 0;
		},
	};
}
