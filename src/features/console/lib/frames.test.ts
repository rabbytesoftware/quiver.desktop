import { beforeAll, describe, expect, it, vi } from 'vitest';

import { fieldKind, formatLogTime, jsonLines, parseExecFrame, parseLogFrame, toLevel } from './frames';

beforeAll(() => {
	vi.stubEnv('TZ', 'UTC');
});

const LOG = {
	type: 'log',
	seq: 412,
	time: '2026-10-04T14:02:14.390123Z',
	level: 'warn',
	component: 'release',
	msg: 'channel lookup slow',
	fields: { ns: 'github.com/char2cs/crowbar', took: '1.8s', retry: 1, ok: false, err: 'boom' },
	fields_truncated: false,
};

describe('fieldKind', () => {
	it('types by JSON type first', () => {
		expect(fieldKind('retry', 1)).toBe('number');
		expect(fieldKind('ok', true)).toBe('bool');
		expect(fieldKind('err', 3)).toBe('number');
	});

	it('marks the error keys', () => {
		expect(fieldKind('err', 'boom')).toBe('error');
		expect(fieldKind('error', 'boom')).toBe('error');
	});

	it.each(['1.8s', '38ms', '1m30s', '2h', '500µs', '3ns'])('%s is a duration', (v) =>
		expect(fieldKind('took', v)).toBe('duration')
	);

	it.each(['~/.quiver/quiver.sock', '/tmp/x', 'C:\\Users\\x', 'C:/x', 'github.com/char2cs/crowbar'])(
		'%s is a path',
		(v) => expect(fieldKind('p', v)).toBe('path')
	);

	it.each(['nightly', '1.8', 'sha256:9f2c', '10 s', '', 's'])('%s is text', (v) =>
		expect(fieldKind('k', v)).toBe('text')
	);

	it('treats non-scalar values as text', () => {
		expect(fieldKind('k', { a: 1 })).toBe('text');
		expect(fieldKind('k', null)).toBe('text');
	});
});

describe('toLevel', () => {
	it.each([
		['debug', 'debug'],
		['TRACE', 'debug'],
		['info', 'info'],
		['warn', 'warn'],
		['warning', 'warn'],
		['error', 'error'],
		['fatal', 'error'],
		['panic', 'error'],
		['loud', 'info'],
		[undefined, 'info'],
		[3, 'info'],
	])('%s → %s', (raw, want) => expect(toLevel(raw)).toBe(want));
});

describe('parseLogFrame', () => {
	it('decodes a log frame from text', () => {
		const frame = parseLogFrame(JSON.stringify(LOG));
		expect(frame.type).toBe('log');
		if (frame.type !== 'log') return;
		expect(frame.record).toMatchObject({
			seq: 412,
			iso: LOG.time,
			level: 'warn',
			component: 'release',
			msg: 'channel lookup slow',
			fieldsTruncated: false,
		});
		expect(frame.record.fields).toEqual([
			{ key: 'ns', value: 'github.com/char2cs/crowbar', kind: 'path' },
			{ key: 'took', value: '1.8s', kind: 'duration' },
			{ key: 'retry', value: '1', kind: 'number' },
			{ key: 'ok', value: 'false', kind: 'bool' },
			{ key: 'err', value: 'boom', kind: 'error' },
		]);
	});

	it('accepts an already-parsed object', () => {
		expect(parseLogFrame(LOG).type).toBe('log');
	});

	it('defaults the optional parts of a log frame', () => {
		const frame = parseLogFrame({ type: 'log', seq: 1, msg: 'hi' });
		expect(frame).toEqual({
			type: 'log',
			record: { seq: 1, iso: '', level: 'info', component: '', msg: 'hi', fields: [], fieldsTruncated: false },
		});
	});

	it('flags truncated fields', () => {
		const frame = parseLogFrame({ ...LOG, fields_truncated: true });
		expect(frame.type === 'log' && frame.record.fieldsTruncated).toBe(true);
	});

	it('stringifies nested values rather than throwing', () => {
		const frame = parseLogFrame({ type: 'log', seq: 1, msg: 'm', fields: { a: { b: 1 }, n: null } });
		expect(frame.type === 'log' && frame.record.fields.map((f) => f.value)).toEqual(['{"b":1}', 'null']);
	});

	it('decodes ready and gap', () => {
		expect(parseLogFrame('{"type":"ready","seq":9}')).toEqual({ type: 'ready', seq: 9, reset: false });
		expect(parseLogFrame('{"type":"ready","seq":3,"reset":true}')).toEqual({ type: 'ready', seq: 3, reset: true });
		expect(parseLogFrame('{"type":"ready","seq":3,"reset":"yes"}')).toEqual({
			type: 'ready',
			seq: 3,
			reset: false,
		});
		expect(parseLogFrame('{"type":"gap","dropped":37}')).toEqual({ type: 'gap', dropped: 37 });
	});

	it('turns anything it does not understand into a raw frame, never an error', () => {
		const bad: unknown[] = [
			'not json',
			'',
			'[1,2]',
			'null',
			'42',
			'{"type":"log"}',
			'{"type":"log","seq":"1","msg":"m"}',
			'{"type":"ready"}',
			'{"type":"gap","dropped":"x"}',
			'{"type":"something-new"}',
			'{}',
			[1, 2],
			null,
			7,
		];
		for (const input of bad) {
			expect(parseLogFrame(input).type, JSON.stringify(input)).toBe('raw');
		}
	});

	it('keeps the original text of a raw frame', () => {
		expect(parseLogFrame('not json')).toEqual({ type: 'raw', text: 'not json' });
		expect(parseLogFrame('{"type":"new"}')).toEqual({ type: 'raw', text: '{"type":"new"}' });
	});
});

describe('formatLogTime', () => {
	it('is the clock time with milliseconds', () => {
		expect(formatLogTime('2026-10-04T14:02:14.390123Z')).toBe('14:02:14.390');
	});

	it('returns what it cannot read', () => {
		expect(formatLogTime('')).toBe('');
		expect(formatLogTime('yesterday')).toBe('yesterday');
	});
});

describe('jsonLines', () => {
	it('lays the record out as the daemon would print it, strings quoted', () => {
		const frame = parseLogFrame(LOG);
		if (frame.type !== 'log') throw new Error('expected a log frame');
		expect(jsonLines(frame.record).map((l) => [l.key, l.value])).toEqual([
			['time', '"2026-10-04T14:02:14.390123Z"'],
			['level', '"warn"'],
			['component', '"release"'],
			['msg', '"channel lookup slow"'],
			['ns', '"github.com/char2cs/crowbar"'],
			['took', '"1.8s"'],
			['retry', '1'],
			['ok', 'false'],
			['err', '"boom"'],
		]);
	});

	it('omits an absent component', () => {
		const frame = parseLogFrame({ type: 'log', seq: 1, msg: 'm' });
		if (frame.type !== 'log') throw new Error('expected a log frame');
		expect(jsonLines(frame.record).map((l) => l.key)).toEqual(['time', 'level', 'msg']);
	});
});

describe('parseExecFrame', () => {
	it('decodes output', () => {
		expect(parseExecFrame('{"type":"out","stream":"stdout","data":"hi\\n"}')).toEqual({
			type: 'out',
			stream: 'stdout',
			data: 'hi\n',
		});
		expect(parseExecFrame('{"type":"out","stream":"stderr","data":"x"}')).toMatchObject({ stream: 'stderr' });
	});

	it('treats an unknown stream as stdout', () => {
		expect(parseExecFrame('{"type":"out","stream":"fd3","data":"x"}')).toMatchObject({ stream: 'stdout' });
	});

	it('decodes exit and error', () => {
		expect(parseExecFrame('{"type":"exit","code":2,"error":"boom"}')).toEqual({
			type: 'exit',
			code: 2,
			error: 'boom',
		});
		expect(parseExecFrame('{"type":"exit","code":0}')).toEqual({ type: 'exit', code: 0, error: '' });
		expect(parseExecFrame('{"type":"error","status":403,"message":"no"}')).toEqual({
			type: 'error',
			status: 403,
			message: 'no',
		});
		expect(parseExecFrame('{"type":"error"}')).toEqual({ type: 'error', status: 0, message: '' });
	});

	it('turns anything else into a raw frame', () => {
		for (const bad of ['nope', '[]', '1', '{"type":"out"}', '{"type":"exit","code":"1"}', '{"type":"new"}', '{}']) {
			expect(parseExecFrame(bad).type, bad).toBe('raw');
		}
	});
});
