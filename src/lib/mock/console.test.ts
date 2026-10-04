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
		expect(frames).toEqual([
			{ type: 'error', status: 403, message: 'command "daemon" is not available in the console' },
		]);
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
		expect(last(frames)).toEqual({ type: 'exit', code: 2, error: 'missing namespace' });
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

	it('wants a namespace to uninstall, and says which when it has one', () => {
		expect(JSON.stringify(run('uninstall').frames)).toContain('this arrow');
		expect(JSON.stringify(run('uninstall --yes').frames)).toContain('uninstalled');
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
