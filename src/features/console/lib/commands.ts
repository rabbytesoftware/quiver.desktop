/**
 * The daemon's own command table, as `GET /v0/console/commands` reports it.
 *
 * This client has no list of commands of its own: help and Tab completion are
 * both read from here, so the grammar is versioned with the daemon that runs it.
 */

export interface ConsoleCommand {
	/** `['arrow', 'list']`. */
	path: string[];
	short: string;
	usage: string;
}

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function strings(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : [];
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

/**
 * Tab completion over command words only: it completes `ins` to `install` and
 * `arrow li` to `arrow list`. Arguments and flags belong to the daemon, so it
 * stops completing words at the first one that is not part of the command path.
 */
export function completeLine(line: string, commands: readonly ConsoleCommand[]): Completion {
	const endsInSpace = /\s$/.test(line);
	const words = line.trim() === '' ? [] : line.trim().split(/\s+/);
	const typed = endsInSpace || words.length === 0 ? '' : (words[words.length - 1] ?? '');
	const before = endsInSpace || words.length === 0 ? words : words.slice(0, -1);

	const candidates = new Set<string>();
	for (const { path } of commands) {
		if (path.length <= before.length || !before.every((word, i) => path[i] === word)) continue;
		const next = path[before.length];
		if (next.startsWith(typed)) candidates.add(next);
	}

	const matches = [...candidates].sort();
	if (matches.length === 0) return { line, candidates: [] };

	const completed = matches.length === 1 ? `${matches[0]} ` : commonPrefix(matches);
	const head = before.length > 0 ? `${before.join(' ')} ` : '';
	return { line: `${head}${completed}`, candidates: matches };
}
