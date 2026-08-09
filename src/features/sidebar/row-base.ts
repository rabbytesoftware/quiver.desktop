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
 * `relative z-10` puts every row ABOVE the travelling indicator, which is
 * `z-0` on the rail. The row's own background is transparent, so the indicator
 * shows through it while the label and icon stay on top.
 *
 * `tracking-[-0.1px]` is design.pen's own number and NOT part of the port —
 * Crowbar's base sets no tracking. Kept because it is Quiver's type spec rather
 * than CossUI's structure, and left off, the whole rail reads a shade looser
 * than every mock it was drawn against.
 */
export const ROW_BASE =
	'relative z-10 flex cursor-pointer select-none items-center gap-1.5 rounded-lg border ' +
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
 * Selection INVERTS, and the fill that does it is NOT here — it belongs to
 * `RailIndicator`, one box that travels the whole rail. This constant only
 * carries what the row itself owns: its text colour, and the timing that keeps
 * that colour legible while the box is in flight.
 *
 * The inversion is the one place the port deliberately departs from Crowbar.
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
	'data-[status=active]:text-background ' +
	// Asymmetric on purpose, and this is the detail that makes the travel read.
	//
	// The fill is no longer the row's — it is one box sliding across the rail —
	// so a row that flips to `--background` the instant it is selected has
	// near-black text on the dark rail for the whole journey, and the label
	// disappears until the box lands. Delaying only the ARRIVAL keeps both ends
	// legible: the row being left returns to `--foreground` immediately, and the
	// row being entered waits for the box to get there.
	//
	// The delay is the transition's duration; `transition-colors` on ROW_BASE
	// would apply it in both directions, which is the bug.
	'data-[status=active]:[transition:color_0ms_200ms]';

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
 * need the room. The token is the same one the design gives this slot, so the
 * box follows it rather than restating a number.
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

/**
 * The travelling fill itself — one box, shared by every selectable surface in
 * the rail, rendered by `RailIndicator`.
 *
 * It lives here rather than in that component because this file IS the rail's
 * selection language: what a selected surface looks like, and how its text
 * behaves while the fill is on its way. Splitting the two halves across files
 * is how they drift.
 *
 * `z-0` puts it above the nav track's background — unpositioned, so it paints
 * in the block layer — and below every segment and row, which carry `z-10`.
 *
 * `transform` and `opacity` are compositor properties. `width` is not, and it
 * is the one thing here that costs layout: affordable because an absolutely
 * positioned box reflows nothing but itself, for the dozen or so frames a
 * transition lasts. `height` never actually changes — a nav segment and an
 * arrow row are both 36px.
 *
 * Three suppressions, all load-bearing:
 *   `ready`       the first paint, or it flies in from the rail's corner
 *   `scrolling`   a transition fed a new transform per frame smears behind
 *   motion-reduce the user asked for no movement
 */
export const RAIL_INDICATOR = [
	'pointer-events-none absolute top-0 left-0 z-0 rounded-lg',
	'bg-foreground shadow-xs shadow-black/10 inset-shadow-[0_1px_var(--selected-edge)]',
	'will-change-transform transition-[transform,width,opacity] duration-200',
	// `ease-out` leaves at full speed and decelerates the whole way, which reads
	// as a lurch at the start — most of the distance is covered in the first
	// third. This curve eases in as well, so the fastest part is the MIDDLE:
	// per-frame steps are small at both ends, and small steps are what the eye
	// reads as smooth. Same 200ms; only the distribution changes.
	//
	// It also hides the one rough edge here. `transform` is composited and
	// interpolates on its own thread, `width` does not — so when the main thread
	// is busy (a selection change re-renders the list) the width lags the
	// position and the box appears to stretch. Small deltas at the ends make
	// that lag far harder to see.
	'ease-[cubic-bezier(0.4,0,0.2,1)]',
	'data-[visible=false]:opacity-0',
	'not-data-[ready=true]:transition-none data-[scrolling=true]:transition-none',
	'motion-reduce:transition-none',
].join(' ');
