import type { SocketLike } from '@/lib/transport/backend';
import type { ConsoleRun } from '@/lib/transport/console-exec';

import type { SocketHub } from './socket';
import type { Clock } from './world/types';

/**
 * A faithful stand-in for the daemon's console surface (`docs/console-spec.md`):
 * `GET /versions` advertising `console.v1`, `GET /v0/console/commands`, the
 * `GET /v0/console/logs` stream and `POST /v0/console/exec`.
 *
 * Faithful means it keeps the contract's edges, not just its happy path: the
 * exec endpoint refuses a command that is not in its table with the same status
 * a daemon answers, ends every accepted run with one `exit` frame, and the log
 * stream replays from a cursor and then says `ready`.
 */

export const MOCK_CORE_VERSIONS = {
	version: 'nightly-latest',
	build_id: '178',
	commit: '9dd0b183177a64ec71a2672d1cd7cf0c70bb4877',
	built_at: '2026-10-04T13:47:00Z',
	channel: 'nightly',
	features: ['console.v1'],
	api: { supported: ['v0'], latest: 'v0' },
};

export interface MockConsoleCommand {
	path: string[];
	short: string;
	usage: string;
	aliases: string[];
}

export const MOCK_COMMANDS: MockConsoleCommand[] = [
	{ path: ['install'], short: 'install an arrow', usage: 'install <namespace>[@ref]', aliases: [] },
	{ path: ['uninstall'], short: 'uninstall an arrow', usage: 'uninstall <namespace> [--yes]', aliases: [] },
	{ path: ['info'], short: 'show an arrow', usage: 'info <namespace>', aliases: [] },
	{ path: ['list'], short: 'list installed arrows', usage: 'list', aliases: ['ls'] },
	{ path: ['search'], short: 'search the catalog', usage: 'search <query>', aliases: [] },
	{ path: ['version'], short: 'show the daemon version', usage: 'version', aliases: [] },
];

interface LogSeed {
	level: 'debug' | 'info' | 'warn' | 'error';
	component: string;
	msg: string;
	fields: Record<string, unknown>;
}

const SEED: LogSeed[] = [
	{
		level: 'info',
		component: 'daemon',
		msg: 'daemon listening',
		fields: { addr: '~/.quiver/quiver.sock', pid: 48213 },
	},
	{ level: 'info', component: 'catalog', msg: 'catalog loaded', fields: { arrows: 5, took: '38ms' } },
	{ level: 'debug', component: 'topic', msg: 'subscribed', fields: { topic: 'quiver-arrow', peers: 2 } },
	{
		level: 'info',
		component: 'arrow',
		msg: 'registered',
		fields: { ns: 'github.com/rabbytesoftware/quiver.desktop', ref: 'nightly-rolling', auto: true },
	},
	{
		level: 'warn',
		component: 'release',
		msg: 'channel lookup slow',
		fields: { ns: 'github.com/char2cs/crowbar', took: '1.8s', retry: 1 },
	},
	{
		level: 'info',
		component: 'arrow',
		msg: 'update available',
		fields: { ns: 'github.com/example/tidepool', from: 'v1.1.4', to: 'v1.2.0' },
	},
	{
		level: 'error',
		component: 'release',
		msg: 'checksum mismatch',
		fields: {
			ns: 'github.com/example/wavelength',
			want: 'sha256:9f2c…e1',
			got: 'sha256:41ab…07',
			err: 'asset rejected',
		},
	},
	{
		level: 'info',
		component: 'arrow',
		msg: 'install rolled back',
		fields: { ns: 'github.com/example/wavelength', took: '3ms' },
	},
];

const LIVE: LogSeed[] = [
	{ level: 'debug', component: 'topic', msg: 'heartbeat', fields: { peers: 2 } },
	{ level: 'info', component: 'catalog', msg: 'catalog refreshed', fields: { arrows: 5, took: '21ms' } },
	{
		level: 'info',
		component: 'runtime',
		msg: 'state changed',
		fields: { ns: 'github.com/char2cs/crowbar', state: 'ready' },
	},
];

const REPLAY_DEFAULT = 500;
const REPLAY_MAX = 2000;
const LIVE_INTERVAL_MS = 4000;

const LEVEL_RANK = { debug: 0, info: 1, warn: 2, error: 3 } as const;

/** Splits on whitespace, honouring double and single quotes. Not a shell: nothing else is special. */
export function tokenise(line: string): string[] | null {
	const tokens: string[] = [];
	let current = '';
	let quote: string | null = null;
	let started = false;
	for (const ch of line) {
		if (quote) {
			if (ch === quote) quote = null;
			else current += ch;
		} else if (ch === '"' || ch === "'") {
			quote = ch;
			started = true;
		} else if (/\s/.test(ch)) {
			if (started || current) tokens.push(current);
			current = '';
			started = false;
		} else {
			current += ch;
		}
	}
	if (quote) return null;
	if (started || current) tokens.push(current);
	return tokens;
}

function frame(value: unknown): string {
	return JSON.stringify(value);
}

function out(data: string, stream: 'stdout' | 'stderr' = 'stdout'): string {
	return frame({ type: 'out', stream, data });
}

function exit(code: number, error = ''): string {
	return frame({ type: 'exit', code, error });
}

function deny(status: number, message: string): string {
	return frame({ type: 'error', status, message });
}

export interface MockConsole {
	commands(): MockConsoleCommand[];
	openLogs(path: string): SocketLike;
	exec(line: string, onFrame: (frame: string) => void): ConsoleRun;
}

export function createMockConsole(hub: SocketHub, clock: Clock): MockConsole {
	let seq = 0;
	const ring: { seq: number; time: string; seed: LogSeed }[] = [];

	function record(seed: LogSeed): { seq: number; time: string; seed: LogSeed } {
		const entry = { seq: ++seq, time: new Date().toISOString(), seed };
		ring.push(entry);
		if (ring.length > 5000) ring.shift();
		return entry;
	}
	for (const seed of SEED) record(seed);

	function logFrame(entry: { seq: number; time: string; seed: LogSeed }): string {
		return frame({
			type: 'log',
			seq: entry.seq,
			time: entry.time,
			level: entry.seed.level,
			component: entry.seed.component,
			msg: entry.seed.msg,
			fields: entry.seed.fields,
			fields_truncated: false,
		});
	}

	let liveIndex = 0;
	let stopLive: (() => void) | null = null;
	const listeners = new Set<(frame: string, level: LogSeed['level']) => void>();

	function publish(seed: LogSeed): void {
		const entry = record(seed);
		const text = logFrame(entry);
		listeners.forEach((send) => send(text, seed.level));
	}

	function ensureLive(): void {
		if (stopLive) return;
		stopLive = clock.every(LIVE_INTERVAL_MS, () => {
			publish(LIVE[liveIndex++ % LIVE.length]);
		});
	}

	return {
		commands: () => MOCK_COMMANDS,

		openLogs(path) {
			const query = new URLSearchParams(path.split('?')[1] ?? '');
			const min = LEVEL_RANK[(query.get('level') as keyof typeof LEVEL_RANK | null) ?? 'info'] ?? 1;
			const since = query.has('since') ? Number(query.get('since')) : null;
			const replay = Math.min(Number(query.get('replay') ?? REPLAY_DEFAULT) || REPLAY_DEFAULT, REPLAY_MAX);

			const socket = hub.open(path);
			const send = (text: string, level: LogSeed['level']): void => {
				if (LEVEL_RANK[level] >= min) socket.deliver(text);
			};

			// Queued after the socket's own open microtask, so the consumer has
			// seen `onopen` -- and set its handlers -- before the first frame.
			queueMicrotask(() => {
				const eligible = ring.filter((e) => LEVEL_RANK[e.seed.level] >= min);
				const replayed =
					since === null ? eligible.slice(-replay) : eligible.filter((e) => e.seq > since).slice(-replay);
				for (const entry of replayed) socket.deliver(logFrame(entry));
				socket.deliver(frame({ type: 'ready', seq }));
				listeners.add(send);
				ensureLive();
			});
			const close = socket.close.bind(socket);
			socket.close = (info) => {
				listeners.delete(send);
				close(info);
			};
			return socket;
		},

		exec(line, onFrame) {
			let cancelled = false;
			const cancel = (): void => {
				cancelled = true;
			};
			const emit = (text: string, delay: number): void => {
				clock.after(delay, () => {
					if (!cancelled) onFrame(text);
				});
			};

			if (line.length > 1024) {
				emit(deny(400, 'command line is too long'), 0);
				return { cancel };
			}
			const tokens = tokenise(line);
			if (tokens === null) {
				emit(deny(400, 'unterminated quote'), 0);
				return { cancel };
			}
			if (tokens.length === 0) {
				emit(deny(400, 'empty command'), 0);
				return { cancel };
			}
			if (tokens.some((t) => /^--(server|context|config)(=|$)/.test(t))) {
				emit(deny(403, `flag "${tokens.find((t) => t.startsWith('--'))}" is not available in the console`), 0);
				return { cancel };
			}

			const [name, ...args] = tokens;
			const known = MOCK_COMMANDS.find((c) => c.path[0] === name || c.aliases.includes(name));
			if (!known) {
				emit(deny(403, `command "${name}" is not available in the console`), 0);
				return { cancel };
			}

			const start = Date.now();
			const finish = (code: number, error: string, delay: number): void => {
				emit(exit(code, error), delay);
				clock.after(delay, () => {
					publish({
						level: 'info',
						component: 'console',
						msg: 'exec',
						fields: { device: 'local', line, code, took: `${Date.now() - start}ms` },
					});
				});
			};

			switch (known.path[0]) {
				case 'version':
					emit(out(`quiver ${MOCK_CORE_VERSIONS.version} (build ${MOCK_CORE_VERSIONS.build_id})\n`), 40);
					finish(0, '', 60);
					break;
				case 'list':
					emit(out('NAMESPACE                                   VERSION           STATE\n'), 40);
					emit(out('github.com/char2cs/crowbar                  v0.4.1            ready\n'), 80);
					emit(out('github.com/example/tidepool                 v1.1.4            outdated\n'), 120);
					finish(0, '', 140);
					break;
				case 'install':
					if (!args[0]) {
						emit(out('usage: install <namespace>[@ref]\n', 'stderr'), 20);
						finish(2, 'missing namespace', 40);
						break;
					}
					emit(out(`resolving ${args[0]} …\n`), 60);
					emit(out('fetching manifest\n'), 400);
					emit(out(`installed ${args[0].split('/').pop()}\n`), 900);
					finish(0, '', 920);
					break;
				case 'uninstall':
					if (!args.includes('--yes')) {
						emit(out(`remove ${args[0] ?? 'this arrow'}? [y/N] `, 'stderr'), 20);
						emit(out('aborted: pass --yes to confirm\n', 'stderr'), 40);
						finish(1, 'confirmation required', 60);
					} else {
						emit(out(`uninstalled ${args[0] ?? ''}\n`), 120);
						finish(0, '', 140);
					}
					break;
				case 'search':
					emit(out(`3 results for "${args.join(' ')}"\n`), 80);
					finish(0, '', 100);
					break;
				default:
					emit(out(`${known.usage}\n`), 40);
					finish(0, '', 60);
			}

			return { cancel };
		},
	};
}
