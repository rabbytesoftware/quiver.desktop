import crypto from 'node:crypto';
import http from 'node:http';
import type { Duplex } from 'node:stream';

/**
 * A minimal RFC 6455 client for text frames, over a unix socket or TCP.
 *
 * Node's built-in WebSocket can not dial a unix socket, and the daemon's
 * `/v0/ui/<ns>/ws` route is exactly that on a local install. Writing the
 * handshake here also lets a spec see the status of a REFUSED upgrade (a 401
 * from the bearer gate), which a WebSocket API only reports as "error".
 */
export interface WsTarget {
	socketPath?: string;
	host?: string;
	port?: number;
	path: string;
	headers?: Record<string, string>;
}

export interface UpgradeResult {
	status: number;
	body?: string;
	ws?: TextSocket;
}

export function wsConnect(target: WsTarget, timeoutMs = 10_000): Promise<UpgradeResult> {
	return new Promise((resolve, reject) => {
		const req = http.request({
			socketPath: target.socketPath,
			host: target.host,
			port: target.port,
			path: target.path,
			method: 'GET',
			headers: {
				Host: 'localhost',
				Connection: 'Upgrade',
				Upgrade: 'websocket',
				'Sec-WebSocket-Version': '13',
				'Sec-WebSocket-Key': crypto.randomBytes(16).toString('base64'),
				...target.headers,
			},
		});
		const timer = setTimeout(() => {
			req.destroy();
			reject(new Error(`upgrade of ${target.path} timed out after ${timeoutMs}ms`));
		}, timeoutMs);
		req.on('upgrade', (res, socket, head) => {
			clearTimeout(timer);
			resolve({ status: res.statusCode ?? 101, ws: new TextSocket(socket, head) });
		});
		req.on('response', (res) => {
			const chunks: Buffer[] = [];
			res.on('data', (c: Buffer) => chunks.push(c));
			res.on('end', () => {
				clearTimeout(timer);
				resolve({ status: res.statusCode ?? 0, body: Buffer.concat(chunks).toString('utf8') });
			});
		});
		req.on('error', (err) => {
			clearTimeout(timer);
			reject(err);
		});
		req.end();
	});
}

export class TextSocket {
	readonly messages: string[] = [];
	closed = false;
	private buffer: Buffer;

	constructor(
		private readonly socket: Duplex,
		head: Buffer
	) {
		this.buffer = Buffer.from(head);
		socket.on('data', (chunk: Buffer) => {
			this.buffer = Buffer.concat([this.buffer, chunk]);
			this.drain();
		});
		socket.on('close', () => {
			this.closed = true;
		});
		socket.on('error', () => {
			this.closed = true;
		});
		this.drain();
	}

	private drain(): void {
		for (;;) {
			const b = this.buffer;
			if (b.length < 2) return;
			const opcode = b[0] & 0x0f;
			let len = b[1] & 0x7f;
			let offset = 2;
			if (len === 126) {
				if (b.length < 4) return;
				len = b.readUInt16BE(2);
				offset = 4;
			} else if (len === 127) {
				if (b.length < 10) return;
				len = Number(b.readBigUInt64BE(2));
				offset = 10;
			}
			if (b.length < offset + len) return;
			const payload = b.subarray(offset, offset + len);
			this.buffer = b.subarray(offset + len);
			if (opcode === 0x1) this.messages.push(payload.toString('utf8'));
			else if (opcode === 0x8) {
				this.closed = true;
				this.socket.end();
			} else if (opcode === 0x9) this.write(0xa, payload);
		}
	}

	private write(opcode: number, payload: Buffer): void {
		const mask = crypto.randomBytes(4);
		const len = payload.length;
		const header =
			len < 126
				? Buffer.from([0x80 | opcode, 0x80 | len])
				: Buffer.from([0x80 | opcode, 0x80 | 126, (len >> 8) & 0xff, len & 0xff]);
		const masked = Buffer.alloc(len);
		for (let i = 0; i < len; i++) masked[i] = payload[i] ^ mask[i % 4];
		this.socket.write(Buffer.concat([header, mask, masked]));
	}

	send(text: string): void {
		this.write(0x1, Buffer.from(text, 'utf8'));
	}

	close(): void {
		if (this.closed) return;
		this.write(0x8, Buffer.from([0x03, 0xe8]));
		this.closed = true;
		this.socket.end();
	}
}
