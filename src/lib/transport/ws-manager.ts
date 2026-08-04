import { backend, SOCKET_OPEN, type SocketLike } from './backend';

type Callback = (data: unknown) => void;

interface Channel {
	socket: SocketLike;
	callbacks: Set<Callback>;
	reconnectDelay: number;
}

function createTransport(endpoint: string): SocketLike {
	return backend().openSocket(endpoint);
}

export interface WSManager {
	subscribe(endpoint: string, cb: Callback): () => void;
	send(endpoint: string, data: unknown): void;
}

// `QuiverWebSocket` defers its own teardown when close() is called while
// still CONNECTING — it flags `closed` internally and issues the real
// ws_close once ws_open resolves (see quiver-socket.ts) — so unlike a raw
// browser `WebSocket` the manager never has to wait for `onopen` before
// tearing a socket down. Calling close() directly is always safe here, and
// every `SocketLike` owes the same guarantee.
function closeSocketQuietly(socket: SocketLike): void {
	socket.close();
}

// Frames missed during an outage cannot be recovered from a DTO merge, so
// this is handed to every subscriber after a reconnect: it tells them the
// stream broke and they need to reseed rather than assume nothing was lost.
// Frozen because it's a single shared instance handed to every subscriber on
// every reconnect, forever — a subscriber mutating its own reference (e.g.
// `data.reconnected = false`) would otherwise poison `isReconnectSentinel`
// for every other subscriber and every future reconnect.
export const RECONNECT_SENTINEL = Object.freeze({ reconnected: true });

export function isReconnectSentinel(data: unknown): boolean {
	return typeof data === 'object' && data !== null && (data as { reconnected?: unknown }).reconnected === true;
}

export function createWSManager(): WSManager {
	const channels = new Map<string, Channel>();

	function open(endpoint: string, reconnectDelay = 1000): Channel {
		const ch: Channel = {
			socket: createTransport(endpoint),
			callbacks: new Set(),
			reconnectDelay,
		};

		// A successful connection resets the backoff so the next outage starts
		// from the base delay again.
		ch.socket.onopen = () => {
			ch.reconnectDelay = 1000;
		};

		ch.socket.onmessage = (e) => {
			let parsed: unknown;
			try {
				parsed = JSON.parse(e.data);
			} catch {
				parsed = e.data;
			}
			ch.callbacks.forEach((cb) => cb(parsed));
		};

		ch.socket.onclose = () => {
			if (ch.callbacks.size === 0) return;
			setTimeout(() => {
				// All subscribers may have left while we waited — do not resurrect a
				// socket nobody listens to. Whichever channel still owned this
				// endpoint's map entry was already removed by subscribe()'s
				// unsubscribe path the instant the last callback left (it always
				// deletes synchronously), so there is nothing left to clean up here.
				if (ch.callbacks.size === 0) return;
				// Carry the (doubled) backoff into the new channel so repeated
				// failures actually back off instead of restarting at the base delay.
				const fresh = open(endpoint, Math.min(ch.reconnectDelay * 2, 30_000));
				fresh.callbacks = ch.callbacks;
				channels.set(endpoint, fresh);
				// Tell subscribers the stream was interrupted so they can refetch
				// whatever pushes they may have missed during the outage.
				ch.callbacks.forEach((cb) => cb(RECONNECT_SENTINEL));
			}, ch.reconnectDelay);
		};

		channels.set(endpoint, ch);
		return ch;
	}

	return {
		subscribe(endpoint, cb) {
			const ch = channels.get(endpoint) ?? open(endpoint);
			ch.callbacks.add(cb);
			return () => {
				ch.callbacks.delete(cb);
				if (ch.callbacks.size === 0) {
					// The channel may have been replaced by a reconnect since this
					// subscription was created; close the live socket, not the stale one.
					const current = channels.get(endpoint);
					if (current && current.callbacks === ch.callbacks) {
						closeSocketQuietly(current.socket);
						channels.delete(endpoint);
					} else {
						closeSocketQuietly(ch.socket);
					}
				}
			};
		},

		send(endpoint, data) {
			const ch = channels.get(endpoint);
			if (ch?.socket.readyState === SOCKET_OPEN) {
				ch.socket.send(JSON.stringify(data));
			}
		},
	};
}

export const wsManager = createWSManager();
