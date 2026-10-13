// The four levels this tab offers a control for. The write path (the
// `<Select>` in engine.tsx) only ever emits one of these.
export const LEVELS = ['debug', 'info', 'warn', 'error'] as const;

export type Level = (typeof LEVELS)[number];

// Core accepts a wider vocabulary than this tab writes, and a config file
// hand-edited (or written by an older core) can hold any of it: `trace`,
// `warning`, `fatal`, `panic`. Against the real daemon — never the mock,
// which always boots at `info` — `configured.logger.level` can arrive as one
// of these, and with no matching `SelectItem` the trigger would render the
// raw token with nothing selected. This maps every alias onto the canonical
// level closest to what it means, purely for display.
const ALIASES: Readonly<Record<string, Level>> = {
	trace: 'debug',
	warning: 'warn',
	fatal: 'error',
	panic: 'error',
};

export function normaliseLevel(raw: string): Level {
	if ((LEVELS as readonly string[]).includes(raw)) return raw as Level;
	return ALIASES[raw] ?? 'info';
}
