/**
 * Where the rail's selection indicator has to be, given what it is pointing at.
 *
 * Pure, and deliberately so: jsdom has no layout engine, so a component that
 * measured and positioned in one place would have its arithmetic covered by
 * nothing. Everything that can be got wrong lives here, and the component
 * around it only reads rects and writes styles.
 */

/** The parts of a DOMRect this needs. Narrowed so tests can pass literals. */
export interface Box {
	top: number;
	left: number;
	width: number;
	height: number;
}

export interface IndicatorFrame {
	/** Offsets from the rail's own top-left, for a `translate3d`. */
	x: number;
	y: number;
	width: number;
	height: number;
	/** How much of the indicator is hidden past the list's top edge. */
	clipTop: number;
	/** The same at the bottom. */
	clipBottom: number;
	/** False once the target is entirely outside the list's visible band. */
	visible: boolean;
}

/**
 * @param target the active `<Link>` — a nav segment or an arrow row
 * @param rail the indicator's positioning context
 * @param list the scroller's viewport, or `null` when the target is not inside
 * it. A nav segment is never clipped, so passing `null` says "no band to stay
 * within" rather than "no scrolling right now".
 */
export function indicatorFrame(target: Box, rail: Box, list: Box | null): IndicatorFrame {
	const x = target.left - rail.left;
	const y = target.top - rail.top;
	const frame: IndicatorFrame = {
		x,
		y,
		width: target.width,
		height: target.height,
		clipTop: 0,
		clipBottom: 0,
		visible: true,
	};

	if (list === null) return frame;

	// Both edges measured in the rail's coordinates, so they compare directly
	// against `y` without a second conversion to get wrong.
	const bandTop = list.top - rail.top;
	const bandBottom = bandTop + list.height;

	// Clipped, not hidden. A row scrolled halfway under the nav should have its
	// highlight cut at the list's edge — hiding it outright makes the indicator
	// blink out while part of its row is still on screen.
	frame.clipTop = Math.max(0, bandTop - y);
	frame.clipBottom = Math.max(0, y + target.height - bandBottom);

	// Only once nothing is left. Using `>=` on both: a row resting exactly on
	// the boundary has zero visible pixels, and painting a zero-height sliver
	// there costs a composite for something nobody can see.
	frame.visible = frame.clipTop + frame.clipBottom < target.height;

	return frame;
}

/**
 * The indicator's `y` after the list has scrolled, without measuring anything.
 *
 * This is the whole reason scrolling stays cheap. Reading `getBoundingClientRect`
 * in a scroll handler is fine on its own, but the handler also WRITES a style,
 * and a read after a write in the next frame is what forces synchronous layout.
 * Keeping the scroll path to arithmetic removes the possibility.
 *
 * `baseY` and `baseScroll` are captured together at measure time, so the delta
 * below is always against the scroll position that produced that `y`.
 */
export function scrolledY(baseY: number, baseScroll: number, scrollTop: number): number {
	return baseY - (scrollTop - baseScroll);
}
