/**
 * The daemon's own command table, as `GET /v0/console/commands` reports it.
 *
 * This client has no list of commands of its own: help and Tab completion are
 * both read from here, so the grammar is versioned with the daemon that runs it.
 */

export interface ConsoleFlag {
	/** `yes`, for `--yes`. */
	name: string;
	/** `y`, for `-y`; empty when it has none. */
	shorthand: string;
	usage: string;
	/** The flag is followed by a value (`--output json`). */
	takesValue: boolean;
}

export interface ConsoleCommand {
	/** `['arrow', 'list']`. */
	path: string[];
	short: string;
	usage: string;
	aliases: string[];
	/** The flags the console will accept; the daemon leaves out the ones it refuses. */
	flags: ConsoleFlag[];
}

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function strings(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : [];
}

function flagsOf(value: unknown): ConsoleFlag[] {
	if (!Array.isArray(value)) return [];
	const flags: ConsoleFlag[] = [];
	for (const item of value) {
		if (!isObject(item) || typeof item.name !== 'string' || item.name === '') continue;
		flags.push({
			name: item.name,
			shorthand: typeof item.shorthand === 'string' ? item.shorthand : '',
			usage: typeof item.usage === 'string' ? item.usage : '',
			takesValue: item.takes_value === true,
		});
	}
	return flags;
}

/** Reads the response's `data`; anything malformed is skipped, never fatal. */
export function parseCommands(data: unknown): ConsoleCommand[] {
	const list = isObject(data) ? data.commands : undefined;
	if (!Array.isArray(list)) return [];
	const commands: ConsoleCommand[] = [];
	for (const item of list) {
		if (!isObject(item)) continue;
		const path = strings(item.path);
		if (path.length === 0) continue;
		commands.push({
			path,
			short: typeof item.short === 'string' ? item.short : '',
			usage: typeof item.usage === 'string' && item.usage !== '' ? item.usage : path.join(' '),
			aliases: strings(item.aliases),
			flags: flagsOf(item.flags),
		});
	}
	return commands;
}

/** Help text, one line per command, usage padded to a common width. */
export function renderHelp(commands: readonly ConsoleCommand[]): string[] {
	const width = commands.reduce((w, c) => Math.max(w, c.usage.length), 0);
	return commands.map((c) => (c.short ? `${c.usage.padEnd(width)}  ${c.short}` : c.usage));
}

export interface Completion {
	/** The line with the word under the cursor completed as far as is unambiguous. */
	line: string;
	/** Every command word that matched; more than one means the completion stopped at a fork. */
	candidates: string[];
}

function commonPrefix(words: readonly string[]): string {
	if (words.length === 0) return '';
	let prefix = words[0];
	for (const word of words) {
		while (!word.startsWith(prefix)) prefix = prefix.slice(0, -1);
	}
	return prefix;
}

/** The command a line is addressed to: the longest command path the line begins with, aliases included. */
function addressedCommand(words: readonly string[], commands: readonly ConsoleCommand[]): ConsoleCommand | null {
	let best: { command: ConsoleCommand; length: number } | null = null;
	for (const command of commands) {
		const paths = [command.path, ...command.aliases.map((alias) => [...command.path.slice(0, -1), alias])];
		for (const path of paths) {
			if (path.length > words.length || !path.every((word, i) => words[i] === word)) continue;
			if (best === null || path.length > best.length) best = { command, length: path.length };
		}
	}
	return best?.command ?? null;
}

/** Completes a `-`/`--` word against the addressed command's own flags. */
function completeFlag(
	before: readonly string[],
	typed: string,
	commands: readonly ConsoleCommand[]
): Completion | null {
	const command = addressedCommand(before, commands);
	if (command === null) return null;

	const matches = new Map<string, ConsoleFlag>();
	for (const flag of command.flags) {
		const long = `--${flag.name}`;
		if (long.startsWith(typed)) matches.set(long, flag);
		const short = flag.shorthand ? `-${flag.shorthand}` : '';
		if (short && short.startsWith(typed) && typed !== '--') matches.set(short, flag);
	}
	const names = [...matches.keys()].sort();
	const head = before.length > 0 ? `${before.join(' ')} ` : '';
	if (names.length === 0) return null;
	if (names.length > 1) return { line: `${head}${commonPrefix(names)}`, candidates: names };

	const flag = matches.get(names[0]) as ConsoleFlag;
	return { line: `${head}${names[0]}${flag.takesValue ? '=' : ' '}`, candidates: names };
}

/**
 * Tab completion over command words only, and over the flags of the command a
 * line is addressed to: it completes `ins` to `install` and
 * `arrow li` to `arrow list`, and `arrow remove ns --y` to `--yes`. Arguments
 * belong to the daemon, so it stops completing words at the first one that is
 * not part of the command path.
 */
export function completeLine(line: string, commands: readonly ConsoleCommand[]): Completion {
	const endsInSpace = /\s$/.test(line);
	const words = line.trim() === '' ? [] : line.trim().split(/\s+/);
	const typed = endsInSpace || words.length === 0 ? '' : (words[words.length - 1] ?? '');
	const before = endsInSpace || words.length === 0 ? words : words.slice(0, -1);

	if (typed.startsWith('-')) return completeFlag(before, typed, commands) ?? { line, candidates: [] };

	const candidates = new Set<string>();
	for (const command of commands) {
		const names = [command.path, ...command.aliases.map((alias) => [...command.path.slice(0, -1), alias])];
		for (const path of names) {
			if (path.length <= before.length || !before.every((word, i) => path[i] === word)) continue;
			const next = path[before.length];
			if (next.startsWith(typed)) candidates.add(next);
		}
	}

	const matches = [...candidates].sort();
	if (matches.length === 0) return { line, candidates: [] };

	const completed = matches.length === 1 ? `${matches[0]} ` : commonPrefix(matches);
	const head = before.length > 0 ? `${before.join(' ')} ` : '';
	return { line: `${head}${completed}`, candidates: matches };
}
