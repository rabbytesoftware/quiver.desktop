/**
 * Shell-style command history: Up walks back through what was run, Down walks
 * forward and finally returns to whatever was being typed.
 */

export interface HistoryCursor {
	/** Position in the history being shown, or null when the user's own draft is. */
	index: number | null;
	/** What the user had typed before they first pressed Up. */
	draft: string;
}

export const FRESH_CURSOR: HistoryCursor = { index: null, draft: '' };

/** Adds a line to the history: not blank, and not a repeat of the most recent one. */
export function pushHistory(history: readonly string[], line: string, cap: number): string[] {
	const trimmed = line.trim();
	if (trimmed === '' || history[history.length - 1] === trimmed) return [...history];
	const next = [...history, trimmed];
	return next.length > cap ? next.slice(next.length - cap) : next;
}

export interface HistoryStep {
	cursor: HistoryCursor;
	/** The text the input should now show. */
	text: string;
}

export function stepHistory(
	history: readonly string[],
	cursor: HistoryCursor,
	current: string,
	direction: 'up' | 'down'
): HistoryStep {
	if (history.length === 0) return { cursor, text: current };

	if (direction === 'up') {
		const index = cursor.index === null ? history.length - 1 : Math.max(0, cursor.index - 1);
		const draft = cursor.index === null ? current : cursor.draft;
		return { cursor: { index, draft }, text: history[index] };
	}

	if (cursor.index === null) return { cursor, text: current };
	if (cursor.index >= history.length - 1) return { cursor: { index: null, draft: '' }, text: cursor.draft };
	const index = cursor.index + 1;
	return { cursor: { index, draft: cursor.draft }, text: history[index] };
}
