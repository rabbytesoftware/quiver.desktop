import { beforeEach, describe, expect, it } from 'vitest';

import { BUFFER_CAP, HISTORY_CAP, useConsoleStore } from './console-store';
import type { LogFrame, LogRecord } from '../lib/frames';
import { parseVersions } from '../lib/versions';

function record(seq: number, over: Partial<LogRecord> = {}): LogRecord {
	return {
		seq,
		iso: '2026-10-04T14:02:14Z',
		level: 'info',
		component: 'daemon',
		msg: `m${seq}`,
		fields: [],
		fieldsTruncated: false,
		...over,
	};
}

const log = (seq: number): LogFrame => ({ type: 'log', record: record(seq) });

const store = () => useConsoleStore.getState();

beforeEach(() => {
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
});

describe('ingest', () => {
	it('appends logs in order and remembers the newest seq', () => {
		store().ingest([log(1), log(2), log(3)]);
		expect(store().entries.map((e) => e.kind === 'log' && e.record.seq)).toEqual([1, 2, 3]);
		expect(store().cursor).toBe(3);
	});

	it('gives every entry its own increasing id', () => {
		store().ingest([log(1), log(2)]);
		store().push([{ kind: 'raw', text: 'x' }]);
		expect(store().entries.map((e) => e.id)).toEqual([1, 2, 3]);
	});

	it('drops a record at or behind the cursor', () => {
		store().ingest([log(1), log(2)]);
		store().ingest([log(2), log(2), log(1), log(3)]);
		expect(store().entries).toHaveLength(3);
		expect(store().cursor).toBe(3);
	});

	it('a reset ready discards the daemon lines held, then shows the replay', () => {
		store().ingest([log(40), log(41)]);
		store().ingest([log(1), log(2), { type: 'ready', seq: 2, reset: true }]);

		const entries = store().entries;
		expect(entries.filter((e) => e.kind === 'log').map((e) => e.kind === 'log' && e.record.seq)).toEqual([1, 2]);
		expect(entries.some((e) => e.kind === 'note' && e.note.type === 'restarted')).toBe(true);
		expect(store().cursor).toBe(2);
	});

	it('shows the replay of a restarted daemon even though its seqs are behind the old cursor', () => {
		store().ingest([log(900)]);
		store().ingest([log(1), { type: 'ready', seq: 1, reset: true }]);
		expect(store().entries.filter((e) => e.kind === 'log')).toHaveLength(1);
	});

	it('a reset keeps what the user typed and what it printed', () => {
		store().ingest([log(5)]);
		store().push([
			{ kind: 'cmd', text: 'list', at: 0 },
			{ kind: 'out', stream: 'stdout', text: 'github.com/char2cs/crowbar' },
		]);
		store().ingest([log(1), { type: 'ready', seq: 1, reset: true }]);

		const kinds = store().entries.map((e) => e.kind);
		expect(kinds).toContain('cmd');
		expect(kinds).toContain('out');
		expect(store().entries.filter((e) => e.kind === 'log')).toHaveLength(1);
	});

	it('a reset collapses the open line: it belonged to the old daemon', () => {
		store().ingest([log(1)]);
		store().toggleExpanded(1);
		store().ingest([log(1), { type: 'ready', seq: 1, reset: true }]);
		expect(store().expandedId).toBeNull();
	});

	it('an ordinary ready changes nothing', () => {
		store().ingest([log(1), log(2)]);
		store().ingest([{ type: 'ready', seq: 2, reset: false }]);
		store().ingest([{ type: 'ready', seq: 9, reset: false }]);
		expect(store().entries).toHaveLength(2);
		expect(store().cursor).toBe(2);
	});

	it('does not call the first ready a restart', () => {
		store().ingest([{ type: 'ready', seq: 0, reset: false }]);
		expect(store().entries).toHaveLength(0);
	});

	it('still drops a record at or behind the cursor when the daemon has not restarted', () => {
		store().ingest([log(10), log(11)]);
		store().ingest([log(11), log(10), log(12), { type: 'ready', seq: 12, reset: false }]);
		expect(store().entries.filter((e) => e.kind === 'log')).toHaveLength(3);
	});

	it('shows what the daemon dropped, and what it could not decode', () => {
		store().ingest([
			{ type: 'gap', dropped: 37 },
			{ type: 'raw', text: 'weird' },
		]);
		expect(store().entries.map((e) => e.kind)).toEqual(['note', 'raw']);
		expect(store().entries[0]).toMatchObject({ note: { type: 'gap', dropped: 37 } });
	});

	it('accepts the first record when the cursor is empty, whatever its seq', () => {
		store().ingest([log(900)]);
		expect(store().cursor).toBe(900);
	});

	it('keeps only the newest BUFFER_CAP entries', () => {
		const frames = Array.from({ length: BUFFER_CAP + 250 }, (_, i) => log(i + 1));
		store().ingest(frames);
		expect(store().entries).toHaveLength(BUFFER_CAP);
		const first = store().entries[0];
		expect(first.kind === 'log' && first.record.seq).toBe(251);
		expect(store().cursor).toBe(BUFFER_CAP + 250);
	});

	it('an empty batch changes nothing', () => {
		store().ingest([]);
		expect(store().entries).toEqual([]);
	});
});

describe('push and clear', () => {
	it('push ignores an empty list', () => {
		const before = store().nextId;
		store().push([]);
		expect(store().nextId).toBe(before);
	});

	it('clear empties the buffer and collapses the open line, but keeps the cursor', () => {
		store().ingest([log(1)]);
		store().toggleExpanded(1);
		store().clear();
		expect(store().entries).toEqual([]);
		expect(store().expandedId).toBeNull();
		expect(store().cursor).toBe(1);
	});
});

describe('toggleExpanded', () => {
	it('opens one line at a time and closes it again', () => {
		store().toggleExpanded(3);
		expect(store().expandedId).toBe(3);
		store().toggleExpanded(4);
		expect(store().expandedId).toBe(4);
		store().toggleExpanded(4);
		expect(store().expandedId).toBeNull();
	});
});

describe('adopt', () => {
	it('starts clean for a new connection but keeps the history and the open state', () => {
		store().adopt('local');
		store().setOpen(true);
		store().ingest([log(1)]);
		store().remember('list');
		store().setVersions(parseVersions({ features: ['console.v1'] }));

		store().adopt('remote-1');

		expect(store().connectionId).toBe('remote-1');
		expect(store().entries).toEqual([]);
		expect(store().cursor).toBeNull();
		expect(store().support).toBe('unknown');
		expect(store().versions).toBeNull();
		expect(store().commandsLoaded).toBe(false);
		expect(store().history).toEqual(['list']);
		expect(store().open).toBe(true);
	});

	it('is a no-op for the connection already shown', () => {
		store().adopt('local');
		store().ingest([log(1)]);
		store().adopt('local');
		expect(store().entries).toHaveLength(1);
	});
});

describe('setVersions', () => {
	it('reads the capability off the daemon', () => {
		store().setVersions(parseVersions({ features: ['console.v1'] }));
		expect(store().support).toBe('supported');
		store().setVersions(parseVersions({ version: '1' }));
		expect(store().support).toBe('unsupported');
	});

	it('says unknown, never unsupported, about a daemon it has not heard from', () => {
		store().setVersions(parseVersions({ features: ['console.v1'] }));
		store().setVersions(null);
		expect(store().support).toBe('unknown');
	});
});

describe('open state', () => {
	it('toggles and sets', () => {
		store().toggle();
		expect(store().open).toBe(true);
		store().toggle();
		expect(store().open).toBe(false);
		store().setOpen(true);
		expect(store().open).toBe(true);
	});
});

describe('command line', () => {
	it('remembers what was run and clears the draft', () => {
		store().setDraft('list');
		store().remember('list');
		expect(store().history).toEqual(['list']);
		expect(store().draft).toBe('');
	});

	it('walks the history with Up and Down and restores the draft', () => {
		store().remember('one');
		store().remember('two');
		store().setDraft('half');
		store().stepHistory('up');
		expect(store().draft).toBe('two');
		store().stepHistory('up');
		expect(store().draft).toBe('one');
		store().stepHistory('down');
		store().stepHistory('down');
		expect(store().draft).toBe('half');
	});

	it('typing leaves history navigation', () => {
		store().remember('one');
		store().stepHistory('up');
		store().setDraft('one more');
		expect(store().historyCursor.index).toBeNull();
	});

	it('caps the history', () => {
		for (let i = 0; i < HISTORY_CAP + 10; i++) store().remember(`cmd ${i}`);
		expect(store().history).toHaveLength(HISTORY_CAP);
		expect(store().history[HISTORY_CAP - 1]).toBe(`cmd ${HISTORY_CAP + 9}`);
	});

	it('counts runs, never below zero', () => {
		store().runStarted();
		store().runStarted();
		store().runEnded();
		expect(store().running).toBe(1);
		store().runEnded();
		store().runEnded();
		expect(store().running).toBe(0);
	});

	it('sets the stream state', () => {
		store().setStream('live');
		expect(store().stream).toBe('live');
	});
});
