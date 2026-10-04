import { describe, expect, it } from 'vitest';

import { completeLine, parseCommands, renderHelp, type ConsoleCommand, type ConsoleFlag } from './commands';

const YES: ConsoleFlag = { name: 'yes', shorthand: 'y', usage: 'skip the confirmation', takesValue: false };
const OUTPUT: ConsoleFlag = { name: 'output', shorthand: 'o', usage: 'table|json|yaml', takesValue: true };
const DETACH: ConsoleFlag = { name: 'detach', shorthand: '', usage: 'do not wait', takesValue: false };

const TABLE: ConsoleCommand[] = [
	{ path: ['install'], short: 'install an arrow', usage: 'install <namespace>[@ref]', aliases: [], flags: [OUTPUT] },
	{ path: ['info'], short: 'show an arrow', usage: 'info <namespace>', aliases: [], flags: [OUTPUT] },
	{
		path: ['uninstall'],
		short: 'remove an arrow',
		usage: 'uninstall <namespace>',
		aliases: [],
		flags: [YES, DETACH],
	},
	{ path: ['list'], short: 'list arrows', usage: 'list', aliases: ['ls'], flags: [] },
	{ path: ['arrow', 'list'], short: 'list arrows', usage: 'arrow list', aliases: [], flags: [OUTPUT] },
	{ path: ['arrow', 'add'], short: 'add an arrow', usage: 'arrow add <namespace>', aliases: [], flags: [] },
	{
		path: ['arrow', 'remove'],
		short: 'remove an arrow',
		usage: 'arrow remove <namespace>',
		aliases: [],
		flags: [YES],
	},
	{ path: ['collection', 'list'], short: '', usage: 'collection list', aliases: [], flags: [] },
];

describe('parseCommands', () => {
	it('reads the response data, flags included', () => {
		const parsed = parseCommands({
			commands: [
				{
					path: ['uninstall'],
					short: 's',
					usage: 'uninstall <ns>',
					aliases: ['rm'],
					flags: [{ name: 'yes', shorthand: 'y', usage: 'skip the prompt', takes_value: false }],
				},
			],
		});
		expect(parsed).toEqual([
			{
				path: ['uninstall'],
				short: 's',
				usage: 'uninstall <ns>',
				aliases: ['rm'],
				flags: [{ name: 'yes', shorthand: 'y', usage: 'skip the prompt', takesValue: false }],
			},
		]);
	});

	it('reads a flag that takes a value', () => {
		const [command] = parseCommands({
			commands: [{ path: ['list'], flags: [{ name: 'output', shorthand: 'o', takes_value: true }] }],
		});
		expect(command.flags).toEqual([{ name: 'output', shorthand: 'o', usage: '', takesValue: true }]);
	});

	it('defaults the optional parts: a daemon with no flags field still parses', () => {
		expect(parseCommands({ commands: [{ path: ['arrow', 'list'] }] })).toEqual([
			{ path: ['arrow', 'list'], short: '', usage: 'arrow list', aliases: [], flags: [] },
		]);
	});

	it('skips flags it cannot use', () => {
		const [command] = parseCommands({
			commands: [{ path: ['x'], flags: [null, 3, {}, { name: '' }, { name: 5 }, { name: 'ok' }] }],
		});
		expect(command.flags.map((f) => f.name)).toEqual(['ok']);
		expect(parseCommands({ commands: [{ path: ['x'], flags: 'nope' }] })[0].flags).toEqual([]);
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

describe('completeLine: command words', () => {
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
		expect(result.candidates).toEqual(['arrow', 'collection', 'info', 'install', 'list', 'ls', 'uninstall']);
		expect(result.line).toBe('');
	});

	it('completes a sub-command', () => {
		expect(completeLine('arrow li', TABLE)).toEqual({ line: 'arrow list ', candidates: ['list'] });
		expect(completeLine('arrow ', TABLE).candidates).toEqual(['add', 'list', 'remove']);
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

	it("does not offer the bare add: it is not in the daemon's table", () => {
		expect(completeLine('ad', TABLE)).toEqual({ line: 'ad', candidates: [] });
	});
});

describe('completeLine: flags', () => {
	it('completes a long flag of the addressed command', () => {
		expect(completeLine('uninstall github.com/char2cs/crowbar --ye', TABLE)).toEqual({
			line: 'uninstall github.com/char2cs/crowbar --yes ',
			candidates: ['--yes'],
		});
	});

	it('completes a flag in a nested command', () => {
		expect(completeLine('arrow remove x --y', TABLE).line).toBe('arrow remove x --yes ');
	});

	it('offers a short flag for a lone dash, with the long ones', () => {
		expect(completeLine('uninstall x -', TABLE).candidates).toEqual(['--detach', '--yes', '-y']);
	});

	it('stops at the common prefix of several flags', () => {
		const result = completeLine('uninstall x --', TABLE);
		expect(result.candidates).toEqual(['--detach', '--yes']);
		expect(result.line).toBe('uninstall x --');
	});

	it('does not offer short flags for a double dash', () => {
		expect(completeLine('uninstall x --', TABLE).candidates).not.toContain('-y');
	});

	it('completes a short flag', () => {
		expect(completeLine('uninstall x -y', TABLE)).toEqual({ line: 'uninstall x -y ', candidates: ['-y'] });
	});

	it('leaves a flag that takes a value open for it', () => {
		expect(completeLine('install x --out', TABLE).line).toBe('install x --output=');
		expect(completeLine('install x -o', TABLE).line).toBe('install x -o=');
	});

	it('addresses the command by an alias', () => {
		const table: ConsoleCommand[] = [{ ...TABLE[3], flags: [YES] }];
		expect(completeLine('ls --ye', table).line).toBe('ls --yes ');
	});

	it('uses the longest command path, not just its parent', () => {
		expect(completeLine('arrow add x --y', TABLE)).toEqual({ line: 'arrow add x --y', candidates: [] });
		expect(completeLine('arrow list --o', TABLE).line).toBe('arrow list --output=');
	});

	it('offers no flag a command does not have', () => {
		expect(completeLine('list --y', TABLE)).toEqual({ line: 'list --y', candidates: [] });
		expect(completeLine('install x --yes', TABLE)).toEqual({ line: 'install x --yes', candidates: [] });
	});

	it('offers no flag for something that is no command', () => {
		expect(completeLine('daemon --y', TABLE)).toEqual({ line: 'daemon --y', candidates: [] });
		expect(completeLine('--y', TABLE)).toEqual({ line: '--y', candidates: [] });
	});
});
