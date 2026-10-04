import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useConsoleStore } from '@/features/console/stores/console-store';
import type { Backend, ConsoleRun, SocketLike } from '@/lib/transport/backend';

import type { ConsoleCommand } from './commands';
import { createConsoleController, FLUSH_MS, type ConsoleController } from './controller';
import { parseVersions, type CoreVersions } from './versions';

class FakeSocket implements SocketLike {
	readyState = 1;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onclose: (() => void) | null = null;
	onerror: ((event: unknown) => void) | null = null;
	closed = false;
	constructor(readonly path: string) {}
	send(): void {}
	close(): void {
		this.closed = true;
	}
	deliver(frame: unknown): void {
		this.onmessage?.({ data: typeof frame === 'string' ? frame : JSON.stringify(frame) });
	}
}

interface Exec {
	line: string;
	cancelled: boolean;
	emit: (frame: unknown) => void;
}

const COMMANDS: ConsoleCommand[] = [
	{ path: ['install'], short: 'install an arrow', usage: 'install <namespace>', aliases: [], flags: [] },
	{ path: ['list'], short: 'list arrows', usage: 'list', aliases: [], flags: [] },
];

const SUPPORTED = parseVersions({ version: 'nightly-latest', features: ['console.v1'] }) as CoreVersions;
const LEGACY = parseVersions({ version: '0.1.0' }) as CoreVersions;

interface Rig {
	controller: ConsoleController;
	sockets: FakeSocket[];
	execs: Exec[];
	versions: { value: CoreVersions | null | Error; delay?: Promise<void> };
	commands: { value: ConsoleCommand[] | Error };
	fetchVersions: ReturnType<typeof vi.fn>;
	fetchCommands: ReturnType<typeof vi.fn>;
	sync: (id: string, ready: boolean) => Promise<void>;
	tick: (ms: number) => void;
}

let rig: Rig;
const store = () => useConsoleStore.getState();

function build(options: { syncExec?: boolean } = {}): Rig {
	const sockets: FakeSocket[] = [];
	const execs: Exec[] = [];
	const versions: Rig['versions'] = { value: SUPPORTED };
	const commands: Rig['commands'] = { value: COMMANDS };
	const timers: { fn: () => void; at: number; live: boolean }[] = [];
	let clock = 0;

	const backend = {
		openSocket: (path: string) => {
			const socket = new FakeSocket(path);
			sockets.push(socket);
			return socket;
		},
		execConsole: (line: string, onFrame: (frame: string) => void): ConsoleRun => {
			const exec: Exec = {
				line,
				cancelled: false,
				emit: (frame) => onFrame(typeof frame === 'string' ? frame : JSON.stringify(frame)),
			};
			execs.push(exec);
			if (options.syncExec) exec.emit({ type: 'exit', code: 0, error: '' });
			return {
				cancel: () => {
					exec.cancelled = true;
				},
			};
		},
	} as unknown as Backend;

	const fetchVersions = vi.fn(async () => {
		await versions.delay;
		if (versions.value instanceof Error) throw versions.value;
		return versions.value;
	});
	const fetchCommands = vi.fn(async () => {
		if (commands.value instanceof Error) throw commands.value;
		return commands.value;
	});

	const controller = createConsoleController({
		store: useConsoleStore,
		backend: () => backend,
		fetchVersions,
		fetchCommands,
		now: () => 1_791_122_529_000,
		timers: {
			set: (fn, ms) => {
				const t = { fn, at: clock + ms, live: true };
				timers.push(t);
				return t;
			},
			clear: (handle) => {
				(handle as { live: boolean }).live = false;
			},
		},
	});

	return {
		controller,
		sockets,
		execs,
		versions,
		commands,
		fetchVersions,
		fetchCommands,
		sync: async (id, ready) => {
			controller.sync(id, ready);
			await vi.waitFor(() => expect(true).toBe(true));
			await Promise.resolve();
			await Promise.resolve();
			await Promise.resolve();
		},
		tick: (ms) => {
			clock += ms;
			for (const t of timers) {
				if (t.live && t.at <= clock) {
					t.live = false;
					t.fn();
				}
			}
		},
	};
}

beforeEach(() => {
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
	rig = build();
});

afterEach(() => {
	rig.controller.dispose();
});

describe('noticing whether the daemon has a console', () => {
	it('asks once the connection is ready, and learns the capability and the commands', async () => {
		await rig.sync('local', true);
		expect(store().connectionId).toBe('local');
		expect(store().support).toBe('supported');
		expect(store().versions).toBe(SUPPORTED);
		expect(store().commands).toEqual(COMMANDS);
		expect(store().commandsLoaded).toBe(true);
	});

	it('does not ask a daemon that is not ready', async () => {
		await rig.sync('local', false);
		expect(rig.fetchVersions).not.toHaveBeenCalled();
		expect(store().support).toBe('unknown');
	});

	it('asks again when a daemon that was down comes back', async () => {
		await rig.sync('local', false);
		await rig.sync('local', true);
		await rig.sync('local', true);
		expect(rig.fetchVersions).toHaveBeenCalledTimes(1);
		await rig.sync('local', false);
		await rig.sync('local', true);
		expect(rig.fetchVersions).toHaveBeenCalledTimes(2);
	});

	it('reads an older daemon as unsupported and does not fetch commands it does not have', async () => {
		rig.versions.value = LEGACY;
		await rig.sync('local', true);
		expect(store().support).toBe('unsupported');
		expect(rig.fetchCommands).not.toHaveBeenCalled();
	});

	it('stays unknown, not unsupported, when the daemon cannot be reached', async () => {
		rig.versions.value = new Error('unreachable');
		await rig.sync('local', true);
		expect(store().support).toBe('unknown');
		expect(store().versions).toBeNull();
	});

	it('works without help and completion when the command table cannot be fetched', async () => {
		rig.commands.value = new Error('boom');
		await rig.sync('local', true);
		expect(store().support).toBe('supported');
		expect(store().commandsLoaded).toBe(false);
	});

	it('ignores a slow answer for a connection that is no longer shown', async () => {
		let release: () => void = () => {};
		rig.versions.delay = new Promise((resolve) => (release = resolve));
		await rig.sync('local', true);
		rig.versions.delay = undefined;
		rig.versions.value = LEGACY;
		await rig.sync('remote', true);
		release();
		await Promise.resolve();
		await Promise.resolve();
		await Promise.resolve();

		expect(store().connectionId).toBe('remote');
		expect(store().support).toBe('unsupported');
	});
});

describe('the log stream', () => {
	it('opens only while the console is open, the daemon supports it and it is ready', async () => {
		await rig.sync('local', true);
		expect(rig.sockets).toHaveLength(0);

		store().setOpen(true);
		expect(rig.sockets).toHaveLength(1);
		expect(rig.sockets[0].path).toContain('/v0/console/logs');

		store().setOpen(false);
		expect(rig.sockets[0].closed).toBe(true);
	});

	it('does not open for an unsupported daemon, even when the console is open', async () => {
		rig.versions.value = LEGACY;
		store().setOpen(true);
		await rig.sync('local', true);
		expect(rig.sockets).toHaveLength(0);
	});

	it('opens as soon as support is learned when the console was already open', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		expect(rig.sockets).toHaveLength(1);
	});

	it('closes when the daemon stops being ready, and reopens from the cursor when it is', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 7, msg: 'hi', level: 'info' });
		rig.sockets[0].deliver({ type: 'ready', seq: 7 });
		rig.tick(FLUSH_MS);

		await rig.sync('local', false);
		expect(rig.sockets[0].closed).toBe(true);
		await rig.sync('local', true);
		expect(rig.sockets).toHaveLength(2);
		expect(rig.sockets[1].path).toContain('since=7');
	});

	it('folds frames into the buffer in batches', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		rig.sockets[0].deliver({ type: 'log', seq: 2, msg: 'b', level: 'warn' });
		rig.sockets[0].deliver({ type: 'ready', seq: 2 });
		expect(store().entries).toHaveLength(0);

		rig.tick(FLUSH_MS);
		expect(store().entries).toHaveLength(2);
		expect(store().cursor).toBe(2);
	});

	it('holds a replay back until its ready, so a reset can replace what is shown', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		rig.tick(FLUSH_MS * 4);
		expect(store().entries).toHaveLength(0);

		rig.sockets[0].deliver({ type: 'ready', seq: 1 });
		rig.tick(FLUSH_MS);
		expect(store().entries).toHaveLength(1);
	});

	it('passes live frames straight through once the replay is over', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'ready', seq: 0 });
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'live', level: 'info' });
		rig.tick(FLUSH_MS);
		expect(store().entries).toHaveLength(1);
	});

	it('does not need the socket to have reported open before frames arrive', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		// The bridge can deliver before the open call resolves; nothing here waits for `onopen`.
		expect(rig.sockets[0].onopen).toBeTypeOf('function');
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		rig.sockets[0].deliver({ type: 'ready', seq: 1 });
		rig.tick(FLUSH_MS);
		expect(store().entries).toHaveLength(1);
	});

	it('flushes what is pending when the console closes', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'ready', seq: 0 });
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		store().setOpen(false);
		expect(store().entries).toHaveLength(1);
	});

	it('replaces the shown log with the replay when the daemon says it restarted', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 40, msg: 'old', level: 'info' });
		rig.sockets[0].deliver({ type: 'ready', seq: 40 });
		rig.tick(FLUSH_MS);
		expect(store().cursor).toBe(40);

		// The connection drops; the daemon is back, numbering from 1 again, and says so.
		rig.sockets[0].onclose?.();
		rig.tick(1000);
		expect(rig.sockets[1].path).toContain('since=40');
		rig.sockets[1].deliver({ type: 'log', seq: 1, msg: 'new', level: 'info' });
		rig.sockets[1].deliver({ type: 'ready', seq: 1, reset: true });
		rig.tick(FLUSH_MS);

		const logs = store().entries.filter((e) => e.kind === 'log');
		expect(logs).toHaveLength(1);
		expect(logs[0].kind === 'log' && logs[0].record.msg).toBe('new');
		expect(store().entries.some((e) => e.kind === 'note' && e.note.type === 'restarted')).toBe(true);
		expect(store().cursor).toBe(1);
	});

	it('forgets a replay that was cut off by a dropped connection', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'half', level: 'info' });
		rig.sockets[0].onclose?.();
		rig.tick(1000);
		rig.sockets[1].deliver({ type: 'log', seq: 1, msg: 'whole', level: 'info' });
		rig.sockets[1].deliver({ type: 'ready', seq: 1 });
		rig.tick(FLUSH_MS);

		const logs = store().entries.filter((e) => e.kind === 'log');
		expect(logs.map((e) => e.kind === 'log' && e.record.msg)).toEqual(['whole']);
	});
});

describe('switching connection', () => {
	it('starts clean, stops the stream and cancels what is running', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		rig.tick(FLUSH_MS);
		rig.controller.submit('install x');
		expect(store().running).toBe(1);

		await rig.sync('remote', true);

		expect(rig.sockets[0].closed).toBe(true);
		expect(rig.execs[0].cancelled).toBe(true);
		expect(store().running).toBe(0);
		expect(store().entries).toEqual([]);
		expect(store().connectionId).toBe('remote');
	});

	it('drops frames a cancelled run still delivers', async () => {
		await rig.sync('local', true);
		rig.controller.submit('install x');
		await rig.sync('remote', true);
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'late\n' });
		expect(store().entries).toEqual([]);
	});

	it('drops stream frames queued for the connection it left', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.sockets[0].deliver({ type: 'log', seq: 1, msg: 'a', level: 'info' });
		await rig.sync('remote', true);
		rig.tick(FLUSH_MS * 4);
		expect(store().entries.filter((e) => e.kind === 'log')).toHaveLength(0);
	});
});

describe('running a command', () => {
	beforeEach(async () => {
		await rig.sync('local', true);
	});

	it('echoes the line, sends it exactly as typed, and remembers it', () => {
		rig.controller.submit('  install "a b"  ');
		expect(rig.execs[0].line).toBe('install "a b"');
		expect(store().entries[0]).toMatchObject({ kind: 'cmd', text: 'install "a b"', at: 1_791_122_529_000 });
		expect(store().history).toEqual(['install "a b"']);
		expect(store().running).toBe(1);
	});

	it('ignores a blank line', () => {
		rig.controller.submit('   ');
		expect(rig.execs).toHaveLength(0);
		expect(store().entries).toEqual([]);
		expect(store().history).toEqual([]);
	});

	it('shows output a line at a time, joining a line split across frames', () => {
		rig.controller.submit('install x');
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'resolv' });
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'ing x\nfetch' });
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'ing\n' });
		const lines = store().entries.filter((e) => e.kind === 'out');
		expect(lines.map((e) => e.kind === 'out' && e.text)).toEqual(['resolving x', 'fetching']);
	});

	it('keeps stdout and stderr apart and tags each line', () => {
		rig.controller.submit('install x');
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'a' });
		rig.execs[0].emit({ type: 'out', stream: 'stderr', data: 'b\n' });
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: '\n' });
		const lines = store().entries.filter((e) => e.kind === 'out');
		expect(lines.map((e) => e.kind === 'out' && [e.stream, e.text])).toEqual([
			['stderr', 'b'],
			['stdout', 'a'],
		]);
	});

	it('keeps a blank output line', () => {
		rig.controller.submit('install x');
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'a\n\nb\n' });
		const lines = store().entries.filter((e) => e.kind === 'out');
		expect(lines.map((e) => e.kind === 'out' && e.text)).toEqual(['a', '', 'b']);
	});

	it('strips terminal escapes and carriage returns', () => {
		rig.controller.submit('install x');
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: '\u001b[32mok\u001b[0m\r\n' });
		const line = store().entries.find((e) => e.kind === 'out');
		expect(line).toMatchObject({ text: 'ok' });
	});

	it('flushes an unterminated last line when the command exits', () => {
		rig.controller.submit('install x');
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'last words' });
		rig.execs[0].emit({ type: 'exit', code: 0, error: '' });
		const lines = store().entries.filter((e) => e.kind === 'out');
		expect(lines.map((e) => e.kind === 'out' && e.text)).toEqual(['last words']);
		expect(store().running).toBe(0);
	});

	it('says nothing extra for a clean exit, and something for a failure', () => {
		rig.controller.submit('list');
		rig.execs[0].emit({ type: 'exit', code: 0, error: '' });
		expect(store().entries.filter((e) => e.kind === 'note')).toEqual([]);

		rig.controller.submit('install');
		rig.execs[1].emit({ type: 'exit', code: 2, error: 'missing namespace' });
		expect(store().entries[store().entries.length - 1]).toMatchObject({
			kind: 'note',
			tone: 'error',
			note: { type: 'exit', code: 2, error: 'missing namespace' },
		});
	});

	it("reports a refusal with the daemon's own words", () => {
		rig.controller.submit('daemon');
		rig.execs[0].emit({ type: 'error', status: 403, message: 'command "daemon" is not available in the console' });
		expect(store().entries[store().entries.length - 1]).toMatchObject({
			note: { type: 'refused', status: 403, message: 'command "daemon" is not available in the console' },
		});
		expect(store().running).toBe(0);
	});

	it('reports a failure to reach the daemon as plain text', () => {
		rig.controller.submit('list');
		rig.execs[0].emit({ type: 'error', status: 0, message: 'connect failed: no socket' });
		expect(store().entries[store().entries.length - 1]).toMatchObject({
			note: { type: 'text', text: 'connect failed: no socket' },
		});
	});

	it('shows a frame it cannot decode as raw text', () => {
		rig.controller.submit('list');
		rig.execs[0].emit('???');
		expect(store().entries[store().entries.length - 1]).toMatchObject({ kind: 'raw', text: '???' });
		expect(store().running).toBe(1);
	});

	it('ignores frames after the run has ended', () => {
		rig.controller.submit('list');
		rig.execs[0].emit({ type: 'exit', code: 0, error: '' });
		rig.execs[0].emit({ type: 'out', stream: 'stdout', data: 'late\n' });
		expect(store().entries.filter((e) => e.kind === 'out')).toEqual([]);
		expect(store().running).toBe(0);
	});

	it('counts several runs at once', () => {
		rig.controller.submit('install a');
		rig.controller.submit('install b');
		expect(store().running).toBe(2);
		rig.execs[0].emit({ type: 'exit', code: 0, error: '' });
		expect(store().running).toBe(1);
	});

	it('survives a backend that answers before execConsole returns', () => {
		rig.controller.dispose();
		rig = build({ syncExec: true });
		rig.controller.sync('local', true);
		expect(() => rig.controller.submit('list')).not.toThrow();
		expect(store().running).toBe(0);
	});

	it('refuses to run on a daemon known to have no console, and says so', async () => {
		rig.versions.value = LEGACY;
		await rig.sync('other', true);
		rig.controller.submit('list');
		expect(rig.execs).toHaveLength(0);
		expect(store().entries[store().entries.length - 1]).toMatchObject({ note: { type: 'unsupported' } });
	});

	it('still tries when it has not heard from the daemon yet', async () => {
		useConsoleStore.setState({ support: 'unknown' });
		rig.controller.submit('list');
		expect(rig.execs).toHaveLength(1);
	});
});

describe('returning to the newest line', () => {
	beforeEach(async () => {
		await rig.sync('local', true);
	});

	it('every command brings the log back to its newest line, as a terminal does', () => {
		const before = store().tail;
		rig.controller.submit('install x');
		expect(store().tail).toBe(before + 1);
		rig.controller.submit('clear');
		rig.controller.submit('help');
		expect(store().tail).toBe(before + 3);
	});

	it('a blank line does not', () => {
		const before = store().tail;
		rig.controller.submit('   ');
		expect(store().tail).toBe(before);
	});
});

describe('client-side commands', () => {
	beforeEach(async () => {
		await rig.sync('local', true);
	});

	it('clear empties the buffer without asking the daemon', () => {
		store().push([{ kind: 'raw', text: 'x' }]);
		rig.controller.submit('clear');
		expect(store().entries).toEqual([]);
		expect(rig.execs).toHaveLength(0);
		expect(store().history).toEqual(['clear']);
	});

	it("help lists the daemon's own table, not a list of its own", async () => {
		rig.controller.submit('help');
		await rig.sync('local', true);
		const lines = store().entries.filter((e) => e.kind === 'out');
		expect(lines.map((e) => e.kind === 'out' && e.text)).toEqual([
			'install <namespace>  install an arrow',
			'list                 list arrows',
		]);
		expect(rig.execs).toHaveLength(0);
	});

	it('help fetches the table when it was not loaded', async () => {
		useConsoleStore.setState({ commandsLoaded: false, commands: [] });
		rig.controller.submit('help');
		await rig.sync('local', true);
		expect(rig.fetchCommands).toHaveBeenCalledTimes(2);
		expect(store().entries.filter((e) => e.kind === 'out')).toHaveLength(2);
	});

	it('help says so when the table cannot be had', async () => {
		useConsoleStore.setState({ commandsLoaded: false, commands: [] });
		rig.commands.value = new Error('boom');
		rig.controller.submit('help');
		await rig.sync('local', true);
		expect(store().entries[store().entries.length - 1]).toMatchObject({ note: { type: 'helpUnavailable' } });
	});

	it('"help install" goes to the daemon: only a bare help is ours', () => {
		rig.controller.submit('help install');
		expect(rig.execs[0].line).toBe('help install');
	});
});

describe('dispose', () => {
	it('stops the stream, cancels runs and stops reacting', async () => {
		store().setOpen(true);
		await rig.sync('local', true);
		rig.controller.submit('install x');
		rig.controller.dispose();
		expect(rig.sockets[0].closed).toBe(true);
		expect(rig.execs[0].cancelled).toBe(true);

		store().setOpen(false);
		store().setOpen(true);
		expect(rig.sockets).toHaveLength(1);
	});
});
