/**
 * Every row in the rail, in one place.
 *
 * Ported from crowbar's `web/src/components/layout/workspace-row-base.ts`
 * (develop @ 84630e00) and kept as class STRINGS rather than a component for
 * the same reason it is there: several call sites draw rows — the arrow list,
 * the primary nav, and anything added later — and a change has to reach all of
 * them at once. A row that assembles its own chrome drifts within a release.
 *
 * The geometry is Crowbar's, unchanged. The active treatment is not: see
 * {@link ROW_ACTIVE}.
 */

/**
 * The chrome every row wears, selected or not.
 *
 * `border` is here rather than in the variants because the 1px has to exist in
 * both states. Put it only on the active row and every row below the selection
 * shifts a pixel each time you move through the list — which reads as the list
 * twitching, and is the kind of bug nobody reports because nobody can describe
 * it.
 *
 * `rounded-lg` resolves to `--radius`, so the corner follows the token rather
 * than pinning a literal that would survive the next time the scale moves.
 *
 * `tracking-[-0.1px]` is design.pen's own number and NOT part of the port —
 * Crowbar's base sets no tracking. Kept because it is Quiver's type spec rather
 * than CossUI's structure, and left off, the whole rail reads a shade looser
 * than every mock it was drawn against.
 */
export const ROW_BASE =
	'flex cursor-pointer select-none items-center gap-1.5 rounded-lg border ' +
	'h-9 px-1.5 mx-1.5 my-0.5 text-[13px] font-medium tracking-[-0.1px] outline-none ' +
	'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background';

/**
 * An idle row carries no fill and no visible border — it is a name on the rail.
 * Hover is the only surface it ever grows, and `--accent` is a 4% overlay, not
 * a solid grey, so it composites the same over whatever the rail is showing.
 *
 * Selected by the ROUTER's `data-status`, not by a React branch. Selection is
 * not state in this app (spec §5.1): the router marks the active link and CSS
 * reads it, so there is no second copy to disagree with the URL. That also
 * makes hover EXCLUDE the active row rather than be overridden by it — an
 * override still paints for the frame before it wins, which flickers as the
 * cursor crosses the selected row.
 */
export const ROW_INACTIVE =
	'not-data-[status=active]:border-transparent not-data-[status=active]:text-foreground ' +
	'not-data-[status=active]:hover:bg-accent';

/**
 * Selection INVERTS. This is the one place the port deliberately departs from
 * Crowbar.
 *
 * Crowbar's `ROW_ACTIVE` fills from `--background`, lifting the row toward the
 * content column's colour with an inset white highlight along its top edge.
 * That reads as a one-in-fifteen difference: the same colour family, a shade
 * apart. It suits a tree you drag things around in, where the selected row is
 * one of several things you are tracking.
 *
 * Quiver's rail is a list you navigate BY, and its selected row is the answer
 * to "where am I". A total reversal cannot be missed at any rail width, in
 * either theme, at any scroll position — so the fill comes from `--foreground`
 * and the text from `--background`.
 *
 * The border tracks the fill rather than contrasting with it. A visible outline
 * here would read as focus, which the row can already be in for unrelated
 * reasons, and the app would then have two marks meaning different things and
 * looking the same.
 *
 * No inset highlight either. Crowbar's `inset-shadow-[0_1px_white/16%]` only
 * registers on a dark fill; against an inverted row it is either invisible
 * (dark theme, near-white fill) or a dark line that reads as a cut rather than
 * a lift. The inversion is the whole signal and needs no help.
 */
export const ROW_ACTIVE =
	'data-[status=active]:border-foreground data-[status=active]:bg-foreground ' +
	'data-[status=active]:text-background data-[status=active]:shadow-xs data-[status=active]:shadow-black/10 ' +
	'data-[status=active]:inset-shadow-[0_1px_var(--selected-edge)]';

/**
 * The box every LEADING glyph sits in.
 *
 * Fixed, so a glyph that ever renders at another size cannot move the label:
 * one label position per row type, or a visible wobble runs down the rail's
 * left edge wherever row types interleave.
 *
 * `--icon` (20), Quiver's own token for a list icon, rather than Crowbar's
 * `size-4`. Crowbar's leading mark is a line glyph and 16px carries one fine;
 * Quiver's is an arrow's icon — an image, or a two-letter monogram — and both
 * need the room. The token is the one the design gives this slot, so the box
 * follows it rather than restating a number.
 */
export const ROW_GLYPH_BOX = 'inline-flex size-(--icon) shrink-0 items-center justify-center';

/**
 * The second line, on the ACTIVE row only (spec §5.11).
 *
 * Mono because it carries a namespace and a version, which are identifiers —
 * the one place mono is allowed in this app. Names and navigation are Inter.
 *
 * `opacity-60` rather than `text-muted-foreground`: the muted token is a fixed
 * value picked against the rail, and on an inverted row it lands on the wrong
 * side of the flip and stops being legible. Sixty percent of whatever the row
 * currently inherits is correct in both states.
 *
 * `tabular-nums` so a version's digits do not jitter the pinned tail as it
 * changes.
 */
export const ROW_SUBLABEL = 'flex min-w-0 font-mono text-[10.5px]/[13px] tabular-nums opacity-60';
