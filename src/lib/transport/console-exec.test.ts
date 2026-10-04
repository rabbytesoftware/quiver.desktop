import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

// Partial, via `importOriginal`: only the IPC boundary is replaced. `Channel`
// is wrapped so the test can reach the channel the code under test created.
const channels: { onmessage: (frame: string) => void }[] = [];
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	Channel: class {
		onmessage: (frame: string) => void = () => {};
		constructor() {
			channels.push(this);
		}
	},
}));

import { invoke } from '@tauri-apps/api/core';

import { startConsoleExec } from './console-exec';

const mockInvoke = invoke as MockedFunction<typeof invoke>;

beforeEach(() => {
	channels.length = 0;
	mockInvoke.mockReset();
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('startConsoleExec', () => {
	it('starts the native run with the line exactly as typed', () => {
		mockInvoke.mockResolvedValue(undefined);
		startConsoleExec('install "a b" \'c\'', () => {});

		expect(mockInvoke).toHaveBeenCalledTimes(1);
		const [command, args] = mockInvoke.mock.calls[0] as [string, Record<string, unknown>];
		expect(command).toBe('console_exec');
		expect(args.line).toBe('install "a b" \'c\'');
		expect(typeof args.execId).toBe('string');
		expect(args.onFrame).toBe(channels[0]);
	});

	it('routes each frame from the channel to the caller', () => {
		mockInvoke.mockResolvedValue(undefined);
		const seen: string[] = [];
		startConsoleExec('list', (frame) => seen.push(frame));

		channels[0].onmessage('{"type":"out"}');
		channels[0].onmessage('{"type":"exit"}');
		expect(seen).toEqual(['{"type":"out"}', '{"type":"exit"}']);
	});

	it('reports a failure to start as an error frame', async () => {
		mockInvoke.mockRejectedValue('no active connection');
		const seen: string[] = [];
		startConsoleExec('list', (frame) => seen.push(frame));
		await Promise.resolve();
		await Promise.resolve();

		expect(seen).toHaveLength(1);
		expect(JSON.parse(seen[0])).toEqual({ type: 'error', status: 0, message: 'no active connection' });
	});

	it('cancels by the same id and stops delivering frames', () => {
		mockInvoke.mockResolvedValue(undefined);
		const seen: string[] = [];
		const run = startConsoleExec('list', (frame) => seen.push(frame));
		const started = mockInvoke.mock.calls[0][1] as { execId: string };

		run.cancel();
		run.cancel(); // idempotent
		channels[0].onmessage('{"type":"out"}');

		const cancels = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'console_exec_cancel');
		expect(cancels).toHaveLength(1);
		expect(cancels[0][1]).toEqual({ execId: started.execId });
		expect(seen).toEqual([]);
	});

	it('says nothing about a start that fails after it was cancelled', async () => {
		let reject: (e: unknown) => void = () => {};
		mockInvoke.mockImplementationOnce(() => new Promise((_, r) => (reject = r)));
		mockInvoke.mockResolvedValue(undefined);
		const seen: string[] = [];
		const run = startConsoleExec('list', (frame) => seen.push(frame));
		run.cancel();
		reject('late failure');
		await Promise.resolve();
		await Promise.resolve();

		expect(seen).toEqual([]);
	});

	it('gives every run its own id', () => {
		mockInvoke.mockResolvedValue(undefined);
		startConsoleExec('a', () => {});
		startConsoleExec('b', () => {});
		const ids = mockInvoke.mock.calls.map(([, args]) => (args as { execId: string }).execId);
		expect(new Set(ids).size).toBe(2);
	});
});
