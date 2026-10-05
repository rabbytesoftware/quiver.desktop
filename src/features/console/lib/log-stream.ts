import type { SocketLike } from '@/lib/transport/backend';

import { parseLogFrame, type LogFrame } from './frames';

export const LOGS_PATH = '/v0/console/logs';

const RECONNECT_BASE_MS = 1000;
const RECONNECT_MAX_MS = 30_000;

export type LogStreamState = 'idle' | 'connecting' | 'live' | 'reconnecting';

export interface LogStreamDeps {
	open: (path: string) => SocketLike;
	/** The highest `seq` already shown; the stream asks for what comes after it. */
	cursor: () => number | null;
	onFrames: (frames: LogFrame[]) => void;
	onState: (state: LogStreamState) => void;
	/** Injectable so reconnect timing can be tested without waiting. */
	timers?: {
		set: (fn: () => void, ms: number) => unknown;
		clear: (handle: unknown) => void;
	};
}

export interface LogStream {
	start(): void;
	stop(): void;
	running(): boolean;
}

export function streamPath(cursor: number | null): string {
	const query = new URLSearchParams({ level: 'debug' });
	if (cursor !== null) query.set('since', String(cursor));
	return `${LOGS_PATH}?${query.toString()}`;
}

/**
 * The live log stream: opens `GET /v0/console/logs`, hands frames on as they
 * arrive, and when the connection drops reconnects with backoff from wherever it
 * got to (`since=<cursor>`), so a blip costs nothing and a long outage is
 * replayed from the daemon's ring.
 *
 * It owns no data. What a frame means -- including noticing that the daemon
 * restarted and the cursor no longer applies -- is the store's business.
 */
export function createLogStream(deps: LogStreamDeps): LogStream {
	const timers = deps.timers ?? {
		set: (fn, ms) => setTimeout(fn, ms),
		clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
	};

	let active = false;
	let socket: SocketLike | null = null;
	let generation = 0;
	let delay = RECONNECT_BASE_MS;
	let timer: unknown = null;
	/** True once a connection has dropped or been replaced: later attempts are reconnects. */
	let retrying = false;

	function release(): void {
		generation++;
		if (timer !== null) timers.clear(timer);
		timer = null;
		const current = socket;
		socket = null;
		if (current) {
			// Silenced first: closing fires `onclose`, which must not schedule a reconnect.
			current.onopen = current.onmessage = current.onclose = current.onerror = null;
			current.close();
		}
	}

	function connect(): void {
		const mine = ++generation;
		deps.onState(retrying ? 'reconnecting' : 'connecting');
		const next = deps.open(streamPath(deps.cursor()));
		socket = next;

		next.onopen = () => {
			if (mine !== generation) return;
			delay = RECONNECT_BASE_MS;
			deps.onState('live');
		};
		next.onmessage = ({ data }) => {
			if (mine !== generation) return;
			deps.onFrames([parseLogFrame(data)]);
		};
		next.onclose = () => {
			if (mine !== generation || !active) return;
			socket = null;
			retrying = true;
			deps.onState('reconnecting');
			const wait = delay;
			delay = Math.min(delay * 2, RECONNECT_MAX_MS);
			timer = timers.set(() => {
				timer = null;
				if (active) connect();
			}, wait);
		};
		// A socket error is always followed by a close, which is where it is handled.
		next.onerror = () => {};
	}

	return {
		start() {
			if (active) return;
			active = true;
			retrying = false;
			delay = RECONNECT_BASE_MS;
			connect();
		},
		stop() {
			if (!active) return;
			active = false;
			release();
			deps.onState('idle');
		},
		running: () => active,
	};
}
