import type { SocketLike } from '@/lib/transport/backend';

import type { Emitter } from './world/types';

const CONNECTING = 0;
const OPEN = 1;
const CLOSED = 3;

/**
 * Two behaviours copied from `QuiverWebSocket`, because `wsManager` was written
 * against them: it opens asynchronously (the manager assigns its handlers on
 * the line after construction), and `close()` fires `onclose` synchronously
 * with no `onopen` if it was still connecting.
 */
export class MockWebSocket implements SocketLike {
	readyState = CONNECTING;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onclose: (() => void) | null = null;
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

	/** Frames arrive as raw text, mirroring the real bridge. */
	deliver(text: string): void {
		if (this.readyState !== OPEN) return;
		this.onmessage?.({ data: text });
	}

	/** Both endpoints are transition-only: core pushes and reads nothing back. */
	send(_data: string): void {}

	close(): void {
		if (this.readyState === CLOSED) return;
		this.readyState = CLOSED;
		this.detach(this);
		this.onclose?.();
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
		open(path) {
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
			// Copied before iterating: `wsManager`'s teardown closes the socket
			// synchronously from inside its own handler, mutating this set.
			[...set].forEach((socket) => socket.deliver(text));
		},

		closeAll() {
			[...byPath.values()].flatMap((set) => [...set]).forEach((socket) => socket.close());
			byPath.clear();
		},

		countFor(path) {
			return byPath.get(path)?.size ?? 0;
		},
	};
}
