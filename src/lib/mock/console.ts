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
	channel: 'nightly-latest',
	features: ['console.v1'],
	api: { supported: ['v0'], latest: 'v0' },
};

export interface MockConsoleFlag {
	name: string;
	shorthand: string;
	usage: string;
	takes_value: boolean;
}

export interface MockConsoleCommand {
	path: string[];
	short: string;
	usage: string;
	aliases: string[];
	flags: MockConsoleFlag[];
}

const OUTPUT: MockConsoleFlag = {
	name: 'output',
	shorthand: 'o',
	usage: 'output format: table|json|yaml',
	takes_value: true,
};
const YES: MockConsoleFlag = { name: 'yes', shorthand: 'y', usage: 'confirm without prompting', takes_value: false };
const DETACH: MockConsoleFlag = {
	name: 'detach',
	shorthand: '',
	usage: 'fire the method without waiting',
	takes_value: false,
};

function command(path: string[], short: string, usage: string, flags: MockConsoleFlag[] = []): MockConsoleCommand {
	return { path, short, usage, aliases: [], flags };
}

/**
 * The daemon's real console surface (quiver.core `docs/spec/console.md`): there
 * is no bare `add`, the arrow verbs live under `arrow`, and destructive verbs
 * refuse without `--yes`.
 */
export const MOCK_COMMANDS: MockConsoleCommand[] = [
	command(['install'], 'install an arrow', 'install <namespace>[@ref]', [OUTPUT, DETACH]),
	command(['run'], 'run an arrow', 'run <namespace>', [OUTPUT, DETACH]),
	command(['stop'], 'stop an arrow', 'stop <namespace>', [OUTPUT, DETACH]),
	command(['update'], 'update an arrow', 'update <namespace>', [OUTPUT, DETACH]),
	command(['uninstall'], 'uninstall an arrow', 'uninstall <namespace>', [YES, OUTPUT, DETACH]),
	command(['ps'], 'list running arrows', 'ps', [OUTPUT]),
	command(['status'], 'show the daemon status', 'status', [OUTPUT]),
	{ ...command(['list'], 'list installed arrows', 'list', [OUTPUT]), aliases: ['ls'] },
	command(['search'], 'search the catalog', 'search <query>', [OUTPUT]),
	command(['info'], 'show an arrow', 'info <namespace>', [OUTPUT]),
	command(['methods'], "list an arrow's methods", 'methods <namespace>', [OUTPUT]),
	command(['health'], 'check the daemon', 'health'),
	command(['version'], 'show the daemon version', 'version'),
	command(['arrow', 'add'], 'register an arrow in the catalog', 'arrow add <namespace>', [OUTPUT]),
	command(['arrow', 'remove'], 'remove an arrow from the catalog', 'arrow remove <namespace>', [YES, OUTPUT]),
	command(['arrow', 'refresh'], 'refresh an arrow', 'arrow refresh <namespace>', [OUTPUT]),
	command(['arrow', 'list'], 'list catalogued arrows', 'arrow list', [OUTPUT]),
	command(['arrow', 'show'], 'show a catalogued arrow', 'arrow show <namespace>', [OUTPUT]),
	command(['collection', 'list'], 'list collections', 'collection list', [OUTPUT]),
	command(['collection', 'show'], 'show a collection', 'collection show <name>', [OUTPUT]),
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

/** Whether `tokens` begin with this command's path (or an alias spelling of it). */
function matches(c: MockConsoleCommand, tokens: string[]): boolean {
	const paths = [c.path, ...c.aliases.map((a) => [...c.path.slice(0, -1), a])];
	return paths.some((p) => p.length <= tokens.length && p.every((word, i) => tokens[i] === word));
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
				// A cursor ahead of the newest record means the daemon restarted: the
				// cursor is discarded, the last records are replayed, and `ready` says so.
				const reset = since !== null && since > seq;
				const eligible = ring.filter((e) => LEVEL_RANK[e.seed.level] >= min);
				const replayed =
					since === null || reset
						? eligible.slice(-replay)
						: eligible.filter((e) => e.seq > since).slice(-replay);
				for (const entry of replayed) socket.deliver(logFrame(entry));
				socket.deliver(frame(reset ? { type: 'ready', seq, reset: true } : { type: 'ready', seq }));
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
			const refused = tokens.find((t) => /^--(server|context|config)(=|$)/.test(t));
			if (refused) {
				emit(deny(403, `the ${refused.split('=')[0]} flag is not available in the console`), 0);
				return { cancel };
			}

			// The longest listed path the line begins with; anything else, including a bare
			// namespace (the CLI's root dispatch) and `add`, is not reachable.
			const known = MOCK_COMMANDS.filter((c) => matches(c, tokens)).sort(
				(x, y) => y.path.length - x.path.length
			)[0];
			if (!known) {
				emit(deny(403, 'unknown command'), 0);
				return { cancel };
			}
			if (known.path[0] === 'status' && tokens.some((t) => t === '--watch' || t === '-w')) {
				emit(deny(403, 'the --watch flag is not available in the console'), 0);
				return { cancel };
			}

			const args = tokens.slice(known.path.length).filter((t) => !t.startsWith('-'));
			const confirmed = tokens.some((t) => t === '--yes' || t === '-y');
			const verb = known.path.join(' ');
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
			const needsNamespace = /<namespace>|<query>|<name>/.test(known.usage);
			if (needsNamespace && args.length === 0) {
				emit(out(`usage: ${known.usage}\n`, 'stderr'), 20);
				finish(2, 'missing argument', 40);
				return { cancel };
			}

			switch (verb) {
				case 'version':
					emit(out(`quiver ${MOCK_CORE_VERSIONS.version} (build ${MOCK_CORE_VERSIONS.build_id})\n`), 40);
					finish(0, '', 60);
					break;
				case 'health':
					emit(out('daemon: ok\n'), 40);
					finish(0, '', 60);
					break;
				case 'list':
				case 'arrow list':
					emit(out('NAMESPACE                                   VERSION           STATE\n'), 40);
					emit(out('github.com/char2cs/crowbar                  v0.4.1            ready\n'), 80);
					emit(out('github.com/example/tidepool                 v1.1.4            outdated\n'), 120);
					finish(0, '', 140);
					break;
				case 'install':
				case 'arrow add':
					emit(out(`resolving ${args[0]} …\n`), 60);
					emit(out('fetching manifest\n'), 400);
					emit(out(`${verb === 'install' ? 'installed' : 'added'} ${args[0].split('/').pop()}\n`), 900);
					finish(0, '', 920);
					break;
				case 'uninstall':
				case 'arrow remove':
					// Confirmations never prompt: with no terminal they refuse outright.
					if (!confirmed) {
						emit(out(`${verb} requires --yes/-y when not running interactively\n`, 'stderr'), 20);
						finish(1, `${verb} requires --yes/-y when not running interactively`, 40);
					} else {
						emit(out(`${verb === 'uninstall' ? 'uninstalled' : 'removed'} ${args[0]}\n`), 120);
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
