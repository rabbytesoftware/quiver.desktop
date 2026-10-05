import type { LogRecord } from './frames';

/** Something the console says itself, as data: the view turns it into words. */
export type Note =
	| { type: 'restarted' }
	| { type: 'exit'; code: number; error: string }
	| { type: 'refused'; status: number; message: string }
	| { type: 'unsupported' }
	| { type: 'helpUnavailable' }
	| { type: 'text'; text: string };

export type ConsoleEntry =
	| { id: number; kind: 'log'; record: LogRecord }
	| { id: number; kind: 'raw'; text: string }
	| { id: number; kind: 'cmd'; text: string; at: number }
	| { id: number; kind: 'out'; stream: 'stdout' | 'stderr'; text: string }
	| { id: number; kind: 'note'; tone: 'info' | 'ok' | 'error'; note: Note };

/** An entry before it has been given an id. */
export type NewEntry =
	| { kind: 'log'; record: LogRecord }
	| { kind: 'raw'; text: string }
	| { kind: 'cmd'; text: string; at: number }
	| { kind: 'out'; stream: 'stdout' | 'stderr'; text: string }
	| { kind: 'note'; tone: 'info' | 'ok' | 'error'; note: Note };

/**
 * Whether the connected daemon has a console. `unknown` until `/versions` has
 * answered (or when it cannot be reached), so the UI never claims "unsupported"
 * about a daemon it has simply not heard from.
 */
export type Support = 'unknown' | 'supported' | 'unsupported';

export type StreamState = 'idle' | 'connecting' | 'live' | 'reconnecting';
