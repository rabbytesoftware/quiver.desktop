import { describe, expect, it } from 'vitest';

import { completeLine, parseCommands, renderHelp, type ConsoleCommand } from './commands';

const TABLE: ConsoleCommand[] = [
	{ path: ['install'], short: 'install an arrow', usage: 'install <namespace>[@ref]', aliases: [] },
	{ path: ['info'], short: 'show an arrow', usage: 'info <namespace>', aliases: [] },
	{ path: ['list'], short: 'list arrows', usage: 'list', aliases: ['ls'] },
	{ path: ['arrow', 'list'], short: 'list arrows', usage: 'arrow list', aliases: [] },
	{ path: ['arrow', 'add'], short: 'add an arrow', usage: 'arrow add <namespace>', aliases: [] },
	{ path: ['collection', 'list'], short: '', usage: 'collection list', aliases: [] },
];

describe('parseCommands', () => {
	it('reads the response data', () => {
		const parsed = parseCommands({
			commands: [{ path: ['install'], short: 's', usage: 'install <ns>', aliases: ['i'] }],
		});
		expect(parsed).toEqual([{ path: ['install'], short: 's', usage: 'install <ns>', aliases: ['i'] }]);
	});

	it('defaults the optional parts', () => {
		expect(parseCommands({ commands: [{ path: ['arrow', 'list'] }] })).toEqual([
			{ path: ['arrow', 'list'], short: '', usage: 'arrow list', aliases: [] },
		]);
	});

	it('skips entries it cannot use and never throws', () => {
		expect(parseCommands({ commands: [null, 3, {}, { path: [] }, { path: [1] }, { path: ['ok'] }] })).toHaveLength(
			1
		);
		for (const bad of [null, undefined, 'x', 3, [], {}, { commands: 'no' }]) {
			expect(parseCommands(bad)).toEqual([]);
		}
	});
});

describe('renderHelp', () => {
	it('pads usage to a common width', () => {
		expect(renderHelp(TABLE.slice(0, 3))).toEqual([
			'install <namespace>[@ref]  install an arrow',
			'info <namespace>           show an arrow',
			'list                       list arrows',
		]);
	});

	it('shows just the usage when there is no summary', () => {
		expect(renderHelp([TABLE[5]])).toEqual(['collection list']);
	});

	it('is empty for no commands', () => expect(renderHelp([])).toEqual([]));
});

describe('completeLine', () => {
	it('completes a unique first word and appends a space', () => {
		expect(completeLine('ins', TABLE)).toEqual({ line: 'install ', candidates: ['install'] });
	});

	it('stops at the common prefix of several matches', () => {
		const result = completeLine('i', TABLE);
		expect(result.candidates).toEqual(['info', 'install']);
		expect(result.line).toBe('in');
	});

	it('offers every first word for an empty line, completing nothing', () => {
		const result = completeLine('', TABLE);
		expect(result.candidates).toEqual(['arrow', 'collection', 'info', 'install', 'list', 'ls']);
		expect(result.line).toBe('');
	});

	it('completes a sub-command', () => {
		expect(completeLine('arrow li', TABLE)).toEqual({ line: 'arrow list ', candidates: ['list'] });
		expect(completeLine('arrow ', TABLE).candidates).toEqual(['add', 'list']);
	});

	it('completes aliases', () => {
		expect(completeLine('l', TABLE).candidates).toEqual(['list', 'ls']);
		expect(completeLine('ls', TABLE)).toEqual({ line: 'ls ', candidates: ['ls'] });
	});

	it('does not complete past the command path: arguments belong to the daemon', () => {
		expect(completeLine('install git', TABLE)).toEqual({ line: 'install git', candidates: [] });
		expect(completeLine('install ', TABLE).candidates).toEqual([]);
	});

	it('leaves a line with no match alone', () => {
		expect(completeLine('zzz', TABLE)).toEqual({ line: 'zzz', candidates: [] });
	});

	it('completes nothing without a command table', () => {
		expect(completeLine('ins', [])).toEqual({ line: 'ins', candidates: [] });
	});

	it('collapses repeated spaces when it completes', () => {
		expect(completeLine('arrow   li', TABLE).line).toBe('arrow list ');
	});
});
