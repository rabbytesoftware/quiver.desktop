/** Never below this, whatever the answer size or the window. Spec 9.4. */
export const MIN_TILE = 190;
export const COLUMN_GAP = 12;

/**
 * Column *count* follows the size of the answer, so a thin answer spends the
 * frame on the art rather than leaving it empty (spec 9.4).
 *
 * Expressed as a cap rather than a per-answer tile width. Picking a floor and
 * letting `auto-fill` divide made the outcome depend on the exact grid width:
 * `auto-fill` divides (grid width + gap) by (floor + gap), and the result lands
 * either side of a whole number, so a 250px floor gave four columns at one
 * window size and two 375px tiles at another, silently. This says what is meant
 * -- at most `cap` columns, and never a tile below `MIN_TILE` -- and holds at
 * every width, narrowing on its own in a small window rather than needing a
 * breakpoint.
 */
export function columnCap(total: number): number {
	if (total <= 3) return 2;
	if (total <= 8) return 4;
	return 5;
}

export function columnRule(total: number): string {
	const cap = columnCap(total);
	const gaps = `${(cap - 1) * COLUMN_GAP}px`;
	return `repeat(auto-fill, minmax(max(${MIN_TILE}px, calc((100% - ${gaps}) / ${cap})), 1fr))`;
}
