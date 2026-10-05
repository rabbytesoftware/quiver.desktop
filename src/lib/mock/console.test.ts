import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { BAD_LINE, createMockConsole, MOCK_COMMANDS, MOCK_CORE_VERSIONS, parseLine } from './console';
import { createSocketHub } from './socket';
import { createClock } from './world/clock';

function last<T>(items: T[]): T | undefined {
	return items[items.length - 1];
}

beforeEach(() => {
	vi.useFakeTimers();
});

afterEach(() => {
	vi.useRealTimers();
});

function setup() {
	const clock = createClock();
	const hub = createSocketHub();
	return { clock, hub, mock: createMockConsole(hub, clock) };
}

function run(line: string) {
	const { mock, clock } = setup();
	const frames: unknown[] = [];
	const handle = mock.exec(line, (f) => frames.push(JSON.parse(f)));
	vi.advanceTimersByTime(5000);
	clock.cancelAll();
	return { frames, handle };
}

describe('parseLine', () => {
	it.each([
		['list', ['list']],
		['  install   a  b ', ['install', 'a', 'b']],
		['install github.com/char2cs/crowbar@v1.2.3', ['install', 'github.com/char2cs/crowbar@v1.2.3']],
		['list --server tcp://127.0.0.1:1', ['list', '--server', 'tcp://127.0.0.1:1']],
		['x'.repeat(1024), ['x'.repeat(1024)]],
	])('splits %j on whitespace', (line, want) => expect(parseLine(line)).toEqual(want));

	it.each([
		['empty', ''],
		['only spaces', '   '],
		['over 1024 bytes', 'x'.repeat(1025)],
		['a tab', 'list\tall'],
		['a newline', 'list\nall'],
		['a NUL', 'list\u0000'],
		['a double quote', 'install "a b"'],
		['a single quote', "install 'a'"],
		['a backtick', 'install `id`'],
		['a backslash', 'install a\\b'],
		...[...'$;&|<>(){}*?~#'].map((c): [string, string] => [`a ${c}`, `install a${c}b`]),
	])('refuses a line that is %s: there is no quoting', (_why, line) => expect(parseLine(line)).toBeNull());
});

describe('the mock daemon advertises the console', () => {
	it('lists console.v1 and a command table', () => {
		expect(MOCK_CORE_VERSIONS.features).toContain('console.v1');
		expect(MOCK_COMMANDS.length).toBeGreaterThan(0);
	});

	it('lists a path, a summary and a usage line for each command, and nothing else', () => {
		for (const command of MOCK_COMMANDS) {
			expect(Object.keys(command).sort()).toEqual(['path', 'short', 'usage']);
		}
	});

	it('does not list a bare add', () => {
		expect(MOCK_COMMANDS.map((c) => c.path.join(' '))).not.toContain('add');
	});

	it('reports the channel the way the pipeline does', () => {
		expect(MOCK_CORE_VERSIONS.channel).toBe('nightly-latest');
	});

	it('does not offer commands the contract says are never reachable', () => {
		const names = MOCK_COMMANDS.map((c) => c.path[0]);
		for (const forbidden of ['daemon', 'self-update', 'context', 'auth', 'completion']) {
			expect(names).not.toContain(forbidden);
		}
	});
});

describe('exec', () => {
	it('refuses a command that is not in the table, naming it, with the status a daemon answers', () => {
		expect(run('daemon').frames).toEqual([
			{ type: 'error', status: 403, message: 'command "daemon" is not available in the console' },
		]);
		expect(run('context list').frames).toEqual([
			{ type: 'error', status: 403, message: 'command "context" is not available in the console' },
		]);
	});

	it("refuses a line with quoting or shell characters with the daemon's own message", () => {
		for (const line of ['install "a b"', 'list; id', 'install $(id)', 'list | cat', 'install a\\b']) {
			expect(run(line).frames, line).toEqual([{ type: 'error', status: 400, message: BAD_LINE }]);
		}
		expect(BAD_LINE).toContain('quoting is not supported');
	});

	it('refuses an empty line and an oversized one', () => {
		expect(run('   ').frames[0]).toMatchObject({ type: 'error', status: 400, message: BAD_LINE });
		expect(run('x'.repeat(1025)).frames[0]).toMatchObject({ type: 'error', status: 400, message: BAD_LINE });
	});

	it.each(['list --server tcp://evil:1', 'list --context x', 'list --config=/etc/passwd'])(
		'accepts %s and runs the command as if the flag were not there',
		(line) => {
			const plain = run('list').frames;
			expect(run(line).frames).toEqual(plain);
			expect(last(plain)).toMatchObject({ type: 'exit', code: 0 });
		}
	);

	it('runs at most four commands at once, then refuses with 429 until one finishes', () => {
		const { mock, clock } = setup();
		const answers: unknown[][] = [];
		for (let i = 0; i < 5; i++) {
			const frames: unknown[] = [];
			answers.push(frames);
			mock.exec('install github.com/char2cs/crowbar', (f) => frames.push(JSON.parse(f)));
		}
		vi.advanceTimersByTime(5);
		expect(answers[4]).toEqual([{ type: 'error', status: 429, message: 'too many console commands are running' }]);
		expect(answers[0]).not.toContainEqual(expect.objectContaining({ status: 429 }));

		vi.advanceTimersByTime(5000);
		expect(last(answers[0])).toMatchObject({ type: 'exit', code: 0 });

		// A slot is free again.
		const frames: unknown[] = [];
		mock.exec('list', (f) => frames.push(JSON.parse(f)));
		vi.advanceTimersByTime(5000);
		expect(last(frames)).toMatchObject({ type: 'exit', code: 0 });
		clock.cancelAll();
	});

	it('gives a cancelled command back its slot', () => {
		const { mock, clock } = setup();
		const handles = Array.from({ length: 4 }, () => mock.exec('install github.com/char2cs/crowbar', () => {}));
		handles[0].cancel();
		const frames: unknown[] = [];
		mock.exec('list', (f) => frames.push(JSON.parse(f)));
		vi.advanceTimersByTime(5000);
		expect(frames).not.toContainEqual(expect.objectContaining({ status: 429 }));
		clock.cancelAll();
	});

	it('ends an accepted run with exactly one exit frame, after its output', () => {
		const { frames } = run('install github.com/char2cs/crowbar');
		const types = frames.map((f) => (f as { type: string }).type);
		expect(last(types)).toBe('exit');
		expect(types.filter((t) => t === 'exit')).toHaveLength(1);
		expect(types.filter((t) => t === 'out').length).toBeGreaterThan(1);
		expect(last(frames)).toEqual({ type: 'exit', code: 0, error: '' });
	});

	it('a command that fails exits non-zero with its reason', () => {
		const { frames } = run('install');
		expect(last(frames)).toEqual({ type: 'exit', code: 2, error: 'missing argument' });
	});

	it('answers a confirmation with no, and says to pass --yes', () => {
		const { frames } = run('uninstall github.com/char2cs/crowbar');
		expect(JSON.stringify(frames)).toContain('--yes');
		expect(last(frames)).toMatchObject({ type: 'exit', code: 1 });
		expect(last(run('uninstall github.com/char2cs/crowbar --yes').frames)).toMatchObject({ code: 0 });
	});

	it('answers version and search', () => {
		expect(JSON.stringify(run('version').frames)).toContain('nightly-latest');
		expect(JSON.stringify(run('search crowbar').frames)).toContain('3 results for \\"crowbar\\"');
		expect(last(run('info github.com/char2cs/crowbar').frames)).toMatchObject({ type: 'exit', code: 0 });
	});

	it('wants an argument to uninstall, and uninstalls once told which', () => {
		expect(last(run('uninstall').frames)).toMatchObject({ type: 'exit', code: 2 });
		expect(JSON.stringify(run('uninstall x --yes').frames)).toContain('uninstalled x');
		expect(JSON.stringify(run('uninstall x -y').frames)).toContain('uninstalled x');
	});

	it('never prompts: a destructive command without --yes refuses at once, on stderr', () => {
		for (const line of ['uninstall x', 'arrow remove x']) {
			const { frames } = run(line);
			expect(frames[0]).toMatchObject({ type: 'out', stream: 'stderr' });
			expect(JSON.stringify(frames)).toContain('requires --yes/-y when not running interactively');
			expect(JSON.stringify(frames)).not.toContain('[y/N]');
			expect(last(frames)).toMatchObject({ type: 'exit', code: 1 });
		}
		expect(last(run('arrow remove x --yes').frames)).toMatchObject({ type: 'exit', code: 0 });
	});

	it('has no bare add: the arrow verbs live under arrow', () => {
		expect(run('add github.com/char2cs/crowbar').frames).toEqual([
			{ type: 'error', status: 403, message: 'command "add" is not available in the console' },
		]);
		expect(last(run('arrow add github.com/char2cs/crowbar').frames)).toMatchObject({ type: 'exit', code: 0 });
		expect(JSON.stringify(run('arrow add github.com/char2cs/crowbar').frames)).toContain('added crowbar');
	});

	it('refuses a group that is not a command in itself, and the root dispatch', () => {
		expect(run('arrow').frames[0]).toMatchObject({ type: 'error', status: 403 });
		expect(run('github.com/char2cs/crowbar').frames[0]).toMatchObject({ type: 'error', status: 403 });
	});

	it('does not block status --watch: like the daemon, it just runs', () => {
		expect(last(run('status --watch').frames)).toMatchObject({ type: 'exit', code: 0 });
	});

	it('matches the daemon: the longest listed path wins', () => {
		expect(JSON.stringify(run('arrow list').frames)).toContain('crowbar');
		expect(last(run('collection list').frames)).toMatchObject({ code: 0 });
	});

	it('stops delivering once cancelled', () => {
		const { mock, clock } = setup();
		const frames: string[] = [];
		const handle = mock.exec('install github.com/char2cs/crowbar', (f) => frames.push(f));
		vi.advanceTimersByTime(100);
		handle.cancel();
		const seen = frames.length;
		vi.advanceTimersByTime(5000);
		clock.cancelAll();
		expect(frames).toHaveLength(seen);
	});
});

describe('the log stream', () => {
	function open(path: string) {
		const { mock, hub, clock } = setup();
		const frames: Record<string, unknown>[] = [];
		const socket = mock.openLogs(path);
		socket.onmessage = (e) => frames.push(JSON.parse(e.data));
		return { socket, frames, hub, clock, mock };
	}

	it('replays, then says ready with the newest seq, then goes live', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug');
		await Promise.resolve();
		await Promise.resolve();

		const types = frames.map((f) => f.type);
		const ready = types.indexOf('ready');
		expect(ready).toBeGreaterThan(0);
		expect(types.slice(0, ready).every((t) => t === 'log')).toBe(true);
		expect(frames[ready]).toEqual({ type: 'ready', seq: (frames[ready - 1] as { seq: number }).seq });

		vi.advanceTimersByTime(4100);
		expect(frames.length).toBeGreaterThan(ready + 1);
		expect(last(frames)).toMatchObject({ type: 'log' });

		socket.close();
		clock.cancelAll();
	});

	it('numbers records increasingly', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug');
		await Promise.resolve();
		await Promise.resolve();
		const seqs = frames.filter((f) => f.type === 'log').map((f) => f.seq as number);
		expect(seqs).toEqual([...seqs].sort((a, b) => a - b));
		expect(new Set(seqs).size).toBe(seqs.length);
		socket.close();
		clock.cancelAll();
	});

	it('honours the minimum level', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=warn');
		await Promise.resolve();
		await Promise.resolve();
		const levels = new Set(frames.filter((f) => f.type === 'log').map((f) => f.level));
		expect([...levels].sort()).toEqual(['error', 'warn']);
		socket.close();
		clock.cancelAll();
	});

	it('replays only what is after the cursor', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug&since=6');
		await Promise.resolve();
		await Promise.resolve();
		expect(frames.filter((f) => f.type === 'log').map((f) => f.seq)).toEqual([7, 8]);
		socket.close();
		clock.cancelAll();
	});

	it("speaks only the daemon's frames: a log has no truncation flag and nothing says gap", async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug');
		await Promise.resolve();
		await Promise.resolve();
		vi.advanceTimersByTime(8200);
		const log = frames.find((f) => f.type === 'log') as Record<string, unknown>;
		expect(Object.keys(log).sort()).toEqual(['component', 'fields', 'level', 'msg', 'seq', 'time', 'type']);
		expect(frames.map((f) => f.type)).not.toContain('gap');
		socket.close();
		clock.cancelAll();
	});

	it('ignores a replay parameter: the daemon has none, and a cursor replays everything after it', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug&since=0&replay=1');
		await Promise.resolve();
		await Promise.resolve();
		expect(frames.filter((f) => f.type === 'log').length).toBeGreaterThan(1);
		socket.close();
		clock.cancelAll();
	});

	it('records each command as the daemon audits it: the resolved command, not the line typed', async () => {
		const { mock, clock } = setup();
		const logs = mock.openLogs('/v0/console/logs?level=debug');
		const seen: Record<string, unknown>[] = [];
		logs.onmessage = (e) => seen.push(JSON.parse(e.data));
		await Promise.resolve();
		await Promise.resolve();

		mock.exec('version', () => {});
		mock.exec('daemon', () => {});
		vi.advanceTimersByTime(5000);

		const audit = seen.find((f) => f.type === 'log' && f.msg === 'exec');
		expect(audit).toMatchObject({
			component: 'console',
			level: 'info',
			fields: { device: 'local', command: 'quiver version', code: 0 },
		});
		expect((audit as { fields: Record<string, unknown> }).fields).not.toHaveProperty('line');
		expect(seen.find((f) => f.type === 'log' && f.msg === 'exec denied')).toMatchObject({
			level: 'warn',
			fields: { device: 'local' },
		});
		logs.close();
		clock.cancelAll();
	});

	it('says reset when the cursor is ahead of the daemon, and replays what it has', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug&since=9000');
		await Promise.resolve();
		await Promise.resolve();
		expect(frames.filter((f) => f.type === 'log').length).toBeGreaterThan(0);
		const ready = frames.find((f) => f.type === 'ready');
		expect(ready).toMatchObject({ reset: true });
		socket.close();
		clock.cancelAll();
	});

	it('does not say reset for an ordinary resume', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug&since=6');
		await Promise.resolve();
		await Promise.resolve();
		expect(frames.find((f) => f.type === 'ready')).not.toHaveProperty('reset');
		socket.close();
		clock.cancelAll();
	});

	it('stops sending to a closed socket', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug');
		await Promise.resolve();
		await Promise.resolve();
		socket.close();
		const seen = frames.length;
		vi.advanceTimersByTime(20_000);
		clock.cancelAll();
		expect(frames).toHaveLength(seen);
	});
});
