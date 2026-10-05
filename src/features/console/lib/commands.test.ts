import { describe, expect, it } from 'vitest';

import { completeLine, parseCommands, renderHelp, type ConsoleCommand } from './commands';

const TABLE: ConsoleCommand[] = [
	{ path: ['install'], short: 'install an arrow', usage: 'install <namespace>[@ref]' },
	{ path: ['info'], short: 'show an arrow', usage: 'info <namespace>' },
	{ path: ['uninstall'], short: 'remove an arrow', usage: 'uninstall <namespace>' },
	{ path: ['list'], short: 'list arrows', usage: 'list' },
	{ path: ['arrow', 'list'], short: 'list arrows', usage: 'arrow list' },
	{ path: ['arrow', 'add'], short: 'add an arrow', usage: 'arrow add <namespace>' },
	{ path: ['arrow', 'remove'], short: 'remove an arrow', usage: 'arrow remove <namespace>' },
	{ path: ['collection', 'list'], short: '', usage: 'collection list' },
];

describe('parseCommands', () => {
	it('reads the response data', () => {
		expect(parseCommands({ commands: [{ path: ['install'], short: 's', usage: 'install <ns>' }] })).toEqual([
			{ path: ['install'], short: 's', usage: 'install <ns>' },
		]);
	});

	it('ignores fields it does not know, such as the flags an older core listed', () => {
		const [command] = parseCommands({
			commands: [{ path: ['list'], short: 's', usage: 'list', aliases: ['ls'], flags: [{ name: 'output' }] }],
		});
		expect(command).toEqual({ path: ['list'], short: 's', usage: 'list' });
	});

	it('defaults the optional parts', () => {
		expect(parseCommands({ commands: [{ path: ['arrow', 'list'] }] })).toEqual([
			{ path: ['arrow', 'list'], short: '', usage: 'arrow list' },
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
		expect(renderHelp(TABLE.slice(0, 2))).toEqual([
			'install <namespace>[@ref]  install an arrow',
			'info <namespace>           show an arrow',
		]);
	});

	it('shows just the usage when there is no summary', () => {
		expect(renderHelp([TABLE[7]])).toEqual(['collection list']);
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
		expect(result.candidates).toEqual(['arrow', 'collection', 'info', 'install', 'list', 'uninstall']);
		expect(result.line).toBe('');
	});

	it('completes a sub-command', () => {
		expect(completeLine('arrow li', TABLE)).toEqual({ line: 'arrow list ', candidates: ['list'] });
		expect(completeLine('arrow ', TABLE).candidates).toEqual(['add', 'list', 'remove']);
	});

	it('does not complete past the command path: arguments and flags belong to the daemon', () => {
		expect(completeLine('install git', TABLE)).toEqual({ line: 'install git', candidates: [] });
		expect(completeLine('install ', TABLE).candidates).toEqual([]);
		expect(completeLine('uninstall x --y', TABLE)).toEqual({ line: 'uninstall x --y', candidates: [] });
		expect(completeLine('--y', TABLE)).toEqual({ line: '--y', candidates: [] });
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

	it("does not offer the bare add: it is not in the daemon's table", () => {
		expect(completeLine('ad', TABLE)).toEqual({ line: 'ad', candidates: [] });
	});
});
