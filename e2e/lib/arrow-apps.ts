import { browser } from '@wdio/globals';
import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

import { quiverHome, socketPath } from './paths';
import { wsConnect, type TextSocket, type WsTarget } from './ws-client';

/** The handler's Content-Security-Policy (src-tauri/src/arrow_app/csp.rs ARROW_CSP). */
export const ARROW_CSP =
	"default-src 'self'; script-src 'self' 'unsafe-inline'; " +
	"style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; " +
	"connect-src 'self'; frame-src 'none'; form-action 'self'; base-uri 'self'";

/** The `arrow-app` host the shell registers for a namespace (src-tauri/src/arrow_app/hosts.rs). */
export function arrowHost(namespace: string): string {
	return `${crypto.createHash('sha256').update(namespace).digest('hex').slice(0, 32)}.localhost`;
}

/** `/v0/ui/<ns>/<rest>`, with the namespace as one percent-encoded segment. */
export function uiPath(namespace: string, rest = '/'): string {
	return `/v0/ui/${encodeURIComponent(namespace)}${rest}`;
}

export interface RawResponse {
	status: number;
	headers: http.IncomingHttpHeaders;
	body: string;
}

export interface Endpoint {
	socketPath?: string;
	host?: string;
	port?: number;
}

export function localEndpoint(home: string): Endpoint {
	return { socketPath: socketPath(home) };
}

/** One plain HTTP request to the daemon, unwrapped by nothing. */
export function rawRequest(
	endpoint: Endpoint,
	method: string,
	reqPath: string,
	headers: Record<string, string> = {},
	body?: unknown
): Promise<RawResponse> {
	const data = body === undefined ? undefined : typeof body === 'string' ? body : JSON.stringify(body);
	return new Promise((resolve, reject) => {
		const req = http.request(
			{
				...endpoint,
				method,
				path: reqPath,
				headers: {
					Host: 'localhost',
					...(data
						? { 'Content-Type': typeof body === 'string' ? 'text/markdown' : 'application/json', 'Content-Length': Buffer.byteLength(data) }
						: {}),
					...headers,
				},
			},
			(res) => {
				const chunks: Buffer[] = [];
				res.on('data', (c: Buffer) => chunks.push(c));
				res.on('end', () =>
					resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(chunks).toString('utf8') })
				);
			}
		);
		req.on('error', reject);
		req.setTimeout(30_000, () => req.destroy(new Error(`${method} ${reqPath} timed out`)));
		if (data) req.write(data);
		req.end();
	});
}

export interface ChatMessage {
	type: string;
	username: string;
	content: string;
}

/** A quiver.chat participant speaking to the chat through the daemon's `/v0/ui` proxy. */
export class ChatClient {
	private constructor(
		readonly username: string,
		private readonly ws: TextSocket
	) {}

	static async join(endpoint: Endpoint, namespace: string, username: string, headers: Record<string, string> = {}) {
		const target: WsTarget = { ...endpoint, path: uiPath(namespace, `/ws?username=${encodeURIComponent(username)}`), headers };
		const result = await wsConnect(target);
		if (result.status !== 101 || !result.ws) {
			throw new Error(`chat upgrade for ${username} answered ${result.status}: ${result.body ?? ''}`);
		}
		return new ChatClient(username, result.ws);
	}

	get messages(): ChatMessage[] {
		return this.ws.messages.flatMap((raw) => {
			try {
				return [JSON.parse(raw) as ChatMessage];
			} catch {
				return [];
			}
		});
	}

	get closed(): boolean {
		return this.ws.closed;
	}

	say(content: string): void {
		this.ws.send(JSON.stringify({ type: 'message', content }));
	}

	async waitFor(what: string, predicate: (m: ChatMessage) => boolean, timeoutMs = 20_000): Promise<ChatMessage> {
		const deadline = Date.now() + timeoutMs;
		while (Date.now() < deadline) {
			const hit = this.messages.find(predicate);
			if (hit) return hit;
			await new Promise((r) => setTimeout(r, 100));
		}
		throw new Error(`${this.username} never received ${what}; got ${JSON.stringify(this.messages)}`);
	}

	leave(): void {
		this.ws.close();
	}
}

export interface Listener {
	proto: 'tcp' | 'tcp6';
	address: string;
	port: number;
	pids: number[];
	commands: string[];
}

function socketOwners(): Map<string, number[]> {
	const owners = new Map<string, number[]>();
	for (const entry of fs.readdirSync('/proc')) {
		if (!/^\d+$/.test(entry)) continue;
		let fds: string[] = [];
		try {
			fds = fs.readdirSync(`/proc/${entry}/fd`);
		} catch {
			continue;
		}
		for (const fd of fds) {
			let link = '';
			try {
				link = fs.readlinkSync(`/proc/${entry}/fd/${fd}`);
			} catch {
				continue;
			}
			const m = /^socket:\[(\d+)\]$/.exec(link);
			if (!m) continue;
			const pids = owners.get(m[1]) ?? [];
			if (!pids.includes(Number(entry))) pids.push(Number(entry));
			owners.set(m[1], pids);
		}
	}
	return owners;
}

export function commandOf(pid: number): string {
	try {
		return fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').split('\0').filter(Boolean).join(' ');
	} catch {
		return '';
	}
}

/** Every TCP socket in LISTEN state on this machine, with the processes holding it (read from /proc, as `ss -ltnp` does). */
export function tcpListeners(): Listener[] {
	const owners = socketOwners();
	const listeners: Listener[] = [];
	for (const proto of ['tcp', 'tcp6'] as const) {
		let table = '';
		try {
			table = fs.readFileSync(`/proc/net/${proto}`, 'utf8');
		} catch {
			continue;
		}
		for (const line of table.split('\n').slice(1)) {
			const cols = line.trim().split(/\s+/);
			if (cols.length < 10 || cols[3] !== '0A') continue;
			const [address, port] = cols[1].split(':');
			const pids = owners.get(cols[9]) ?? [];
			listeners.push({ proto, address, port: parseInt(port, 16), pids, commands: pids.map(commandOf) });
		}
	}
	return listeners;
}

/** `ss -ltnp` as text, for the record (empty when ss is not installed). */
export function ssListening(): string {
	try {
		return execFileSync('ss', ['-ltnp'], { encoding: 'utf8' });
	} catch (err) {
		return `ss unavailable: ${(err as Error).message}`;
	}
}

/** Pids whose executable's file name starts with `prefix`, read from /proc/<pid>/exe. */
export function pidsByExe(prefix: string): number[] {
	const pids: number[] = [];
	for (const entry of fs.readdirSync('/proc')) {
		if (!/^\d+$/.test(entry)) continue;
		try {
			if (path.basename(fs.readlinkSync(`/proc/${entry}/exe`)).startsWith(prefix)) pids.push(Number(entry));
		} catch {
			/* gone, or a kernel thread */
		}
	}
	return pids;
}

/** Pids whose command line matches `pattern`. */
export function pidsByCommand(pattern: RegExp): number[] {
	const pids: number[] = [];
	for (const entry of fs.readdirSync('/proc')) {
		if (!/^\d+$/.test(entry) || Number(entry) === process.pid) continue;
		if (pattern.test(commandOf(Number(entry)))) pids.push(Number(entry));
	}
	return pids;
}

export function runDir(home: string): string {
	return path.join(quiverHome(home), 'run');
}

/** `ls -la` of the run directory, for the record. */
export function lsRunDir(home: string): string {
	try {
		return execFileSync('ls', ['-la', runDir(home)], { encoding: 'utf8' });
	} catch (err) {
		return `ls failed: ${(err as Error).message}`;
	}
}

export function socketsInRunDir(home: string): string[] {
	try {
		return fs.readdirSync(runDir(home)).filter((f) => f.endsWith('.sock'));
	} catch {
		return [];
	}
}

export async function until<T>(
	what: string,
	read: () => Promise<T> | T,
	ok: (value: T) => boolean,
	timeoutMs = 60_000,
	intervalMs = 250
): Promise<T> {
	const deadline = Date.now() + timeoutMs;
	let last: T = await read();
	while (!ok(last)) {
		if (Date.now() > deadline) throw new Error(`${what} within ${timeoutMs}ms; last seen: ${JSON.stringify(last)}`);
		await new Promise((r) => setTimeout(r, intervalMs));
		last = await read();
	}
	return last;
}

function resultsFolder(): string | null {
	const dir = process.env.QUIVER_E2E_RESULTS;
	if (!dir) return null;
	const folder = path.join(dir, process.env.QUIVER_E2E_SPEC ?? 'spec');
	fs.mkdirSync(folder, { recursive: true });
	return folder;
}

/** Appends one observation to `evidence.jsonl` beside the screenshots, so the run leaves what it saw. */
export function evidence(check: string, data: unknown): void {
	const folder = resultsFolder();
	const line = JSON.stringify({ ts: new Date().toISOString(), check, data });
	console.log(`[evidence] ${line.slice(0, 2000)}`);
	if (folder) fs.appendFileSync(path.join(folder, 'evidence.jsonl'), `${line}\n`);
}

/** Saves the webview through WebDriver and the whole X display through scrot. */
export async function screens(name: string): Promise<void> {
	const folder = resultsFolder();
	if (!folder) return;
	try {
		await browser.saveScreenshot(path.join(folder, `${name}.png`));
	} catch {
		/* no window at this instant */
	}
	try {
		execFileSync('scrot', ['-o', path.join(folder, `${name}.screen.png`)], { stdio: 'ignore' });
	} catch {
		/* scrot is a box-only extra */
	}
}
