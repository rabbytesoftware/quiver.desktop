import type { SocketLike } from '@/lib/transport/backend';

import type { Emitter } from './world/types';

const CONNECTING = 0;
const OPEN = 1;
const CLOSED = 3;

/**
 * A `SocketLike` fed by the mock world instead of a socket.
 *
 * Two behaviours are copied from `QuiverWebSocket` rather than invented,
 * because `wsManager` was written against them and would misbehave otherwise:
 *
 *  1. It opens ASYNCHRONOUSLY. The manager assigns `onopen`/`onmessage`/
 *     `onclose` on the line after construction, so a socket that announced
 *     itself open from its own constructor would fire into null handlers and
 *     the manager would never reset its reconnect backoff. A microtask is
 *     enough — it lands after the whole synchronous assignment block, and
 *     unlike a timer it does not need fake timers advanced in tests.
 *  2. `close()` fires `onclose` SYNCHRONOUSLY, and a close issued while still
 *     CONNECTING yields no `onopen` at all.
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
			// Closed before the open landed: per the WebSocket contract that is a
			// clean teardown with no `onopen`, not an open followed by a close.
			if (this.readyState === CLOSED) return;
			this.readyState = OPEN;
			this.onopen?.();
		});
	}

	/** Called by the hub. Frames arrive as raw text, mirroring the real bridge. */
	deliver(text: string): void {
		if (this.readyState !== OPEN) return;
		this.onmessage?.({ data: text });
	}

	send(): void {
		// The two endpoints the app subscribes to are transition-only: quiver.core
		// pushes on them and reads nothing back. `wsManager.send` exists for
		// symmetry with the browser API and has no caller, so there is nothing
		// here to route and nothing a mock could usefully do with it.
	}

	close(): void {
		if (this.readyState === CLOSED) return;
		this.readyState = CLOSED;
		this.detach(this);
		this.onclose?.();
	}
}

export interface SocketHub extends Emitter {
	open(path: string): MockWebSocket;
	/** Close every live socket. Used when a world is retired. */
	closeAll(): void;
	/** How many sockets are live on `path`. Tests only. */
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
			// Copied before iterating: a subscriber's handler can unsubscribe, and
			// `wsManager`'s teardown closes the socket synchronously, which mutates
			// this very set mid-iteration.
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
