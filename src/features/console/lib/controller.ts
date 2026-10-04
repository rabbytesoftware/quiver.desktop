import type { Backend, ConsoleRun } from '@/lib/transport/backend';

import { renderHelp, type ConsoleCommand } from './commands';
import type { NewEntry, StreamState, Support } from './entries';
import { parseExecFrame, type LogFrame } from './frames';
import { createLogStream, type LogStream } from './log-stream';
import type { CoreVersions } from './versions';

// Built from the character code: a control character in a regex literal is what `no-control-regex` exists to forbid, and here it is the point.
const ANSI = new RegExp(String.raw`${String.fromCharCode(27)}\[[0-9;?]*[A-Za-z]`, 'g');

/** Frames from the stream are folded into the store in batches, not one render each. */
export const FLUSH_MS = 50;

/**
 * What the controller reads of the console's state and the actions it takes on
 * it. The app's Zustand store satisfies this as it is; the controller is written
 * against the shape, so this file stays pure logic -- no store, no React -- and the
 * store is handed to it where it is wired up (`stores/console-controller.ts`).
 */
export interface ConsoleView {
	open: boolean;
	support: Support;
	connectionId: string | null;
	cursor: number | null;
	commands: ConsoleCommand[];
	commandsLoaded: boolean;
	running: number;

	adopt: (connectionId: string) => void;
	setVersions: (versions: CoreVersions | null) => void;
	setCommands: (commands: ConsoleCommand[]) => void;
	setStream: (state: StreamState) => void;
	ingest: (frames: readonly LogFrame[]) => void;
	push: (entries: readonly NewEntry[]) => void;
	clear: () => void;
	remember: (line: string) => void;
	followTail: () => void;
	runStarted: () => void;
	runEnded: () => void;
}

/** The store as the controller sees it: read it, and hear about changes to it. */
export interface ConsoleStore {
	getState: () => ConsoleView;
	subscribe: (listener: (state: ConsoleView, previous: ConsoleView) => void) => () => void;
}

export interface ControllerDeps {
	store: ConsoleStore;
	backend: () => Backend;
	fetchVersions: () => Promise<CoreVersions | null>;
	fetchCommands: () => Promise<ConsoleCommand[]>;
	now?: () => number;
	timers?: {
		set: (fn: () => void, ms: number) => unknown;
		clear: (handle: unknown) => void;
	};
}

export interface ConsoleController {
	/** The connection or its readiness changed. */
	sync(connectionId: string, ready: boolean): void;
	submit(line: string): void;
	dispose(): void;
}

function clean(text: string): string {
	return text.replace(ANSI, '').replace(/\r$/, '');
}

/**
 * Everything the console does that is not rendering: noticing whether the
 * daemon has a console, keeping the log stream open while the console is, and
 * running commands.
 *
 * It holds no state of its own beyond handles; what the user sees is in the
 * store it is given.
 */
export function createConsoleController(deps: ControllerDeps): ConsoleController {
	const store = deps.store;
	const timers = deps.timers ?? {
		set: (fn, ms) => setTimeout(fn, ms),
		clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
	};
	const now = deps.now ?? Date.now;

	let ready = false;
	/** Bumped on every connection change, so a slow answer for the previous connection is ignored. */
	let epoch = 0;
	let queue: LogFrame[] = [];
	/**
	 * The replay of the connection being made, held back until its `ready`. A
	 * `ready` that says `reset` means the daemon restarted and the replay replaces
	 * what is shown, which cannot be decided until the whole replay is in hand.
	 * Null once the replay is over (live frames pass straight through).
	 */
	let replay: LogFrame[] | null = null;
	let flushTimer: unknown = null;
	const runs = new Set<ConsoleRun>();

	function flush(): void {
		if (flushTimer !== null) timers.clear(flushTimer);
		flushTimer = null;
		if (queue.length === 0) return;
		const frames = queue;
		queue = [];
		store.getState().ingest(frames);
	}

	const stream: LogStream = createLogStream({
		open: (path) => deps.backend().openSocket(path),
		cursor: () => store.getState().cursor,
		onFrames: (frames) => {
			for (const frame of frames) {
				if (replay === null) {
					queue.push(frame);
					continue;
				}
				replay.push(frame);
				if (frame.type === 'ready') {
					queue.push(...replay);
					replay = null;
				}
			}
			if (queue.length > 0) flushTimer ??= timers.set(flush, FLUSH_MS);
		},
		onState: (state) => {
			// Every new socket begins with a replay -- and its frames can beat `onopen`,
			// so this starts at the attempt, not at the open.
			if (state === 'connecting' || state === 'reconnecting') replay = [];
			if (state === 'idle') replay = null;
			store.getState().setStream(state);
		},
		timers,
	});

	function reconcile(): void {
		const s = store.getState();
		const wanted = s.open && s.support === 'supported' && ready;
		if (wanted && !stream.running()) stream.start();
		if (!wanted && stream.running()) {
			flush();
			stream.stop();
		}
	}

	const unsubscribe = store.subscribe((s, prev) => {
		if (s.open !== prev.open || s.support !== prev.support) reconcile();
	});

	function cancelRuns(): void {
		for (const run of runs) run.cancel();
		runs.clear();
		while (store.getState().running > 0) store.getState().runEnded();
	}

	async function refresh(mine: number): Promise<void> {
		let versions: CoreVersions | null;
		try {
			versions = await deps.fetchVersions();
		} catch {
			// Unreachable: keep what is known; the next `ready` tries again.
			return;
		}
		if (mine !== epoch) return;
		store.getState().setVersions(versions);
		if (store.getState().support === 'supported' && !store.getState().commandsLoaded) {
			try {
				const commands = await deps.fetchCommands();
				if (mine === epoch) store.getState().setCommands(commands);
			} catch {
				// Help and completion are conveniences; the console works without them.
			}
		}
		reconcile();
	}

	async function showHelp(mine: number): Promise<void> {
		let s = store.getState();
		if (!s.commandsLoaded) {
			try {
				const commands = await deps.fetchCommands();
				if (mine !== epoch) return;
				store.getState().setCommands(commands);
			} catch {
				store.getState().push([{ kind: 'note', tone: 'error', note: { type: 'helpUnavailable' } }]);
				return;
			}
			s = store.getState();
		}
		store
			.getState()
			.push(renderHelp(s.commands).map((text): NewEntry => ({ kind: 'out', stream: 'stdout', text })));
	}

	function run(line: string, mine: number): void {
		store.getState().runStarted();
		const pending = { stdout: '', stderr: '' };
		let finished = false;

		const emitLines = (stream: 'stdout' | 'stderr', data: string): void => {
			const parts = (pending[stream] + data).split('\n');
			pending[stream] = parts.pop() ?? '';
			store.getState().push(parts.map((text): NewEntry => ({ kind: 'out', stream, text: clean(text) })));
		};
		const flushPending = (): void => {
			for (const stream of ['stdout', 'stderr'] as const) {
				if (pending[stream] !== '') {
					store.getState().push([{ kind: 'out', stream, text: clean(pending[stream]) }]);
					pending[stream] = '';
				}
			}
		};
		const end = (): void => {
			if (finished) return;
			finished = true;
			flushPending();
			if (handle) runs.delete(handle);
			if (mine === epoch) store.getState().runEnded();
		};

		// Null until `execConsole` returns: a backend may answer before it does.
		let handle: ConsoleRun | null = null;
		handle = deps.backend().execConsole(line, (text) => {
			if (mine !== epoch || finished) return;
			const frame = parseExecFrame(text);
			switch (frame.type) {
				case 'out':
					emitLines(frame.stream, frame.data);
					break;
				case 'exit':
					flushPending();
					if (frame.code !== 0) {
						store.getState().push([
							{
								kind: 'note',
								tone: 'error',
								note: { type: 'exit', code: frame.code, error: frame.error },
							},
						]);
					}
					end();
					break;
				case 'error':
					flushPending();
					store.getState().push([
						frame.status > 0
							? {
									kind: 'note',
									tone: 'error',
									note: { type: 'refused', status: frame.status, message: frame.message },
								}
							: { kind: 'note', tone: 'error', note: { type: 'text', text: frame.message } },
					]);
					end();
					break;
				case 'raw':
					store.getState().push([{ kind: 'raw', text: frame.text }]);
					break;
			}
		});
		if (!finished) runs.add(handle);
	}

	return {
		sync(connectionId, isReady) {
			const changed = store.getState().connectionId !== connectionId;
			const becameReady = isReady && !ready;
			ready = isReady;
			if (changed) {
				epoch++;
				flush();
				stream.stop();
				cancelRuns();
				queue = [];
				replay = null;
				store.getState().adopt(connectionId);
			}
			if (isReady && (changed || becameReady)) void refresh(epoch);
			reconcile();
		},

		submit(line) {
			const text = line.trim();
			if (text === '') return;
			const s = store.getState();
			// Running a command shows its result: whatever the log was scrolled to, it
			// returns to the newest line, as a terminal does.
			s.followTail();
			s.remember(text);
			if (text === 'clear') {
				s.clear();
				return;
			}
			s.push([{ kind: 'cmd', text, at: now() }]);
			if (text === 'help') {
				void showHelp(epoch);
				return;
			}
			if (s.support === 'unsupported') {
				s.push([{ kind: 'note', tone: 'error', note: { type: 'unsupported' } }]);
				return;
			}
			run(text, epoch);
		},

		dispose() {
			unsubscribe();
			stream.stop();
			cancelRuns();
			if (flushTimer !== null) timers.clear(flushTimer);
			flushTimer = null;
			queue = [];
			replay = null;
		},
	};
}
