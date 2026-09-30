import { Channel, invoke } from '@tauri-apps/api/core';

import type { SocketCloseInfo } from './backend';

export const WS_CLOSE_SENTINEL = '\u0000quiver-ws-close';
const CLOSE_INFO_SEPARATOR = '\u0000';

/** The bridge appends `\0code\0reason` to the sentinel when the daemon's close frame carried them. */
function parseCloseInfo(text: string): SocketCloseInfo | undefined {
	const [, code, ...reason] = text.slice(WS_CLOSE_SENTINEL.length).split(CLOSE_INFO_SEPARATOR);
	const parsed = Number(code);
	if (!code || !Number.isInteger(parsed)) return undefined;
	return { code: parsed, reason: reason.join(CLOSE_INFO_SEPARATOR) };
}

export class QuiverWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSED = 3;

	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onclose: ((info?: SocketCloseInfo) => void) | null = null;
	onerror: ((event: unknown) => void) | null = null;

	readyState: number = QuiverWebSocket.CONNECTING;

	private readonly connId: string;
	private closed = false;

	constructor(path: string) {
		this.connId = crypto.randomUUID();

		const channel = new Channel<string>();
		channel.onmessage = (text) => {
			if (text.startsWith(WS_CLOSE_SENTINEL)) {
				if (this.readyState === QuiverWebSocket.CLOSED) return;
				this.readyState = QuiverWebSocket.CLOSED;
				this.onclose?.(parseCloseInfo(text));
				return;
			}
			this.onmessage?.({ data: text });
		};

		invoke('ws_open', { connId: this.connId, path, onMessage: channel })
			.then(() => {
				if (this.closed) {
					void invoke('ws_close', { connId: this.connId });
					return;
				}
				this.readyState = QuiverWebSocket.OPEN;
				this.onopen?.();
			})
			.catch((err) => {
				this.readyState = QuiverWebSocket.CLOSED;
				if (this.closed) return;
				this.onerror?.(err);
				this.onclose?.();
			});
	}

	send(data: string): void {
		void invoke('ws_send', { connId: this.connId, data });
	}

	close(): void {
		const wasConnecting = this.readyState === QuiverWebSocket.CONNECTING;
		this.readyState = QuiverWebSocket.CLOSED;
		this.closed = true;
		if (!wasConnecting) {
			void invoke('ws_close', { connId: this.connId });
		}
		this.onclose?.();
	}
}
