import { create } from 'zustand';

import type { ConsoleCommand } from '../lib/commands';
import type { LogFrame, LogRecord } from '../lib/frames';
import { FRESH_CURSOR, pushHistory, stepHistory, type HistoryCursor } from '../lib/history';
import { appendCapped } from '../lib/ring';
import { supportsConsole, type CoreVersions } from '../lib/versions';

/** The most lines kept; the oldest go first. Matches the daemon's own ring. */
export const BUFFER_CAP = 5000;

/** The most commands remembered for Up and Down. */
export const HISTORY_CAP = 200;

/** Something the console says itself, as data: the view turns it into words. */
export type Note =
	| { type: 'restarted' }
	| { type: 'gap'; dropped: number }
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

export interface ConsoleState {
	open: boolean;
	/** The connection every field below belongs to. */
	connectionId: string | null;
	support: Support;
	versions: CoreVersions | null;
	commands: ConsoleCommand[];
	commandsLoaded: boolean;
	stream: StreamState;

	entries: ConsoleEntry[];
	nextId: number;
	/** The highest log `seq` shown; the stream resumes after it. */
	cursor: number | null;
	expandedId: number | null;

	draft: string;
	history: string[];
	historyCursor: HistoryCursor;
	/** Commands started and not yet finished. */
	running: number;

	toggle: () => void;
	setOpen: (open: boolean) => void;
	/** Starts a clean slate for `connectionId`; a no-op when it is the one already shown. */
	adopt: (connectionId: string) => void;
	setVersions: (versions: CoreVersions | null) => void;
	setCommands: (commands: ConsoleCommand[]) => void;
	setStream: (state: StreamState) => void;
	/** Folds frames from the log stream into the buffer. Returns true when the daemon turned out to have restarted. */
	ingest: (frames: readonly LogFrame[]) => boolean;
	push: (entries: readonly NewEntry[]) => void;
	clear: () => void;
	toggleExpanded: (id: number) => void;
	setDraft: (draft: string) => void;
	remember: (line: string) => void;
	stepHistory: (direction: 'up' | 'down') => void;
	runStarted: () => void;
	runEnded: () => void;
}

const BLANK = {
	support: 'unknown' as Support,
	versions: null,
	commands: [] as ConsoleCommand[],
	commandsLoaded: false,
	stream: 'idle' as StreamState,
	entries: [] as ConsoleEntry[],
	cursor: null,
	expandedId: null,
	draft: '',
	historyCursor: FRESH_CURSOR,
	running: 0,
};

function numbered(entries: readonly NewEntry[], from: number): ConsoleEntry[] {
	return entries.map((entry, i) => ({ ...entry, id: from + i }) as ConsoleEntry);
}

export const useConsoleStore = create<ConsoleState>((set, get) => ({
	open: false,
	connectionId: null,
	...BLANK,
	nextId: 1,
	history: [],

	toggle: () => set((s) => ({ open: !s.open })),
	setOpen: (open) => set({ open }),

	adopt: (connectionId) => {
		if (get().connectionId === connectionId) return;
		// History is the user's, not the connection's, so it survives a switch.
		set({ connectionId, ...BLANK });
	},

	setVersions: (versions) =>
		set({
			versions,
			support: versions === null ? 'unknown' : supportsConsole(versions) ? 'supported' : 'unsupported',
		}),
	setCommands: (commands) => set({ commands, commandsLoaded: true }),
	setStream: (stream) => set({ stream }),

	ingest: (frames) => {
		let restarted = false;
		let cursor = get().cursor;
		const added: NewEntry[] = [];

		for (const frame of frames) {
			switch (frame.type) {
				case 'log':
					// A reconnect replays from the cursor, but a record can still arrive twice
					// (a replay racing the live feed); the cursor is what makes that harmless.
					if (cursor !== null && frame.record.seq <= cursor) break;
					cursor = frame.record.seq;
					added.push({ kind: 'log', record: frame.record });
					break;
				case 'ready':
					// A daemon that restarted numbers from 1 again, so its newest record is
					// BEHIND the cursor. Resume from nothing, or its first lines are lost.
					if (cursor !== null && frame.seq < cursor) {
						restarted = true;
						cursor = null;
						added.push({ kind: 'note', tone: 'info', note: { type: 'restarted' } });
					}
					break;
				case 'gap':
					added.push({ kind: 'note', tone: 'info', note: { type: 'gap', dropped: frame.dropped } });
					break;
				case 'raw':
					added.push({ kind: 'raw', text: frame.text });
					break;
			}
		}

		set((s) => ({
			cursor,
			nextId: s.nextId + added.length,
			entries: appendCapped(s.entries, numbered(added, s.nextId), BUFFER_CAP),
		}));
		return restarted;
	},

	push: (entries) => {
		if (entries.length === 0) return;
		set((s) => ({
			nextId: s.nextId + entries.length,
			entries: appendCapped(s.entries, numbered(entries, s.nextId), BUFFER_CAP),
		}));
	},

	clear: () => set({ entries: [], expandedId: null }),

	toggleExpanded: (id) => set((s) => ({ expandedId: s.expandedId === id ? null : id })),

	setDraft: (draft) => set({ draft, historyCursor: FRESH_CURSOR }),

	remember: (line) =>
		set((s) => ({ history: pushHistory(s.history, line, HISTORY_CAP), historyCursor: FRESH_CURSOR, draft: '' })),

	stepHistory: (direction) =>
		set((s) => {
			const step = stepHistory(s.history, s.historyCursor, s.draft, direction);
			return { historyCursor: step.cursor, draft: step.text };
		}),

	runStarted: () => set((s) => ({ running: s.running + 1 })),
	runEnded: () => set((s) => ({ running: Math.max(0, s.running - 1) })),
}));
