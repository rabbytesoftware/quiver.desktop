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

function closeSocketQuietly(socket: SocketLike): void {
	socket.close();
}

const RECONNECT_SENTINEL = Object.freeze({ reconnected: true });

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
				if (ch.callbacks.size === 0) return;
				const fresh = open(endpoint, Math.min(ch.reconnectDelay * 2, 30_000));
				fresh.callbacks = ch.callbacks;
				channels.set(endpoint, fresh);
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
