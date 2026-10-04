import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createMockConsole, MOCK_COMMANDS, MOCK_CORE_VERSIONS, tokenise } from './console';
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

describe('tokenise', () => {
	it.each([
		['list', ['list']],
		['  install   a  b ', ['install', 'a', 'b']],
		['install "a b" \'c d\'', ['install', 'a b', 'c d']],
		['x ""', ['x', '']],
		['', []],
		['   ', []],
	])('%j', (line, want) => expect(tokenise(line)).toEqual(want));

	it('refuses an unterminated quote', () => expect(tokenise('install "oops')).toBeNull());

	it('treats shell metacharacters as plain text: it is not a shell', () => {
		expect(tokenise('list; rm -rf / | cat > x $(id) `id` *')).toEqual([
			'list;',
			'rm',
			'-rf',
			'/',
			'|',
			'cat',
			'>',
			'x',
			'$(id)',
			'`id`',
			'*',
		]);
	});
});

describe('the mock daemon advertises the console', () => {
	it('lists console.v1 and a command table', () => {
		expect(MOCK_CORE_VERSIONS.features).toContain('console.v1');
		expect(MOCK_COMMANDS.length).toBeGreaterThan(0);
	});

	it('lists flags, including the confirmation flag on destructive commands', () => {
		const flagsOf = (name: string[]) =>
			MOCK_COMMANDS.find((c) => c.path.join(' ') === name.join(' '))?.flags.map((f) => f.name);
		expect(flagsOf(['uninstall'])).toContain('yes');
		expect(flagsOf(['arrow', 'remove'])).toContain('yes');
		expect(flagsOf(['list'])).toContain('output');
		expect(MOCK_COMMANDS.find((c) => c.path[0] === 'status')?.flags.map((f) => f.name)).not.toContain('watch');
	});

	it('does not list a bare add', () => {
		expect(MOCK_COMMANDS.map((c) => c.path.join(' '))).not.toContain('add');
	});

	it('reports the channel the way the pipeline does', () => {
		expect(MOCK_CORE_VERSIONS.channel).toBe('nightly-latest');
	});

	it('does not offer commands the contract says are never reachable', () => {
		const names = MOCK_COMMANDS.flatMap((c) => [c.path[0], ...c.aliases]);
		for (const forbidden of ['daemon', 'self-update', 'context', 'auth', 'completion']) {
			expect(names).not.toContain(forbidden);
		}
	});
});

describe('exec', () => {
	it('refuses a command that is not in the table, with the status a daemon answers', () => {
		const { frames } = run('daemon');
		expect(frames).toEqual([{ type: 'error', status: 403, message: 'unknown command' }]);
	});

	it.each(['list --server tcp://evil:1', 'list --context x', 'list --config=/etc/passwd'])('refuses %s', (line) => {
		const { frames } = run(line);
		expect(frames).toHaveLength(1);
		expect(frames[0]).toMatchObject({ type: 'error', status: 403 });
	});

	it('refuses an empty line, an unterminated quote, and an oversized line', () => {
		expect(run('   ').frames[0]).toMatchObject({ type: 'error', status: 400 });
		expect(run('install "x').frames[0]).toMatchObject({ type: 'error', status: 400 });
		expect(run('x'.repeat(1025)).frames[0]).toMatchObject({ type: 'error', status: 400 });
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
			{ type: 'error', status: 403, message: 'unknown command' },
		]);
		expect(last(run('arrow add github.com/char2cs/crowbar').frames)).toMatchObject({ type: 'exit', code: 0 });
		expect(JSON.stringify(run('arrow add github.com/char2cs/crowbar').frames)).toContain('added crowbar');
	});

	it('refuses a group that is not a command in itself, and the root dispatch', () => {
		expect(run('arrow').frames[0]).toMatchObject({ type: 'error', status: 403 });
		expect(run('github.com/char2cs/crowbar').frames[0]).toMatchObject({ type: 'error', status: 403 });
	});

	it('refuses status --watch, which would never end', () => {
		for (const line of ['status --watch', 'status -w']) expect(run(line).frames[0]).toMatchObject({ status: 403 });
		expect(last(run('status').frames)).toMatchObject({ type: 'exit', code: 0 });
	});

	it('matches the daemon: the longest listed path wins', () => {
		expect(JSON.stringify(run('arrow list').frames)).toContain('crowbar');
		expect(last(run('collection list').frames)).toMatchObject({ code: 0 });
	});

	it('runs an alias', () => {
		expect(last(run('ls').frames)).toMatchObject({ type: 'exit', code: 0 });
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

	it('caps the replay', async () => {
		const { frames, socket, clock } = open('/v0/console/logs?level=debug&replay=3');
		await Promise.resolve();
		await Promise.resolve();
		expect(frames.filter((f) => f.type === 'log')).toHaveLength(3);
		socket.close();
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
