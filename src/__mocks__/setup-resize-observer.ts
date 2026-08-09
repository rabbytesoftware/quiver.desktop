/**
 * Suite-wide fix, not a design concern of any one test: jsdom ships no
 * `ResizeObserver`. Constructing one throws `ReferenceError`, so any component
 * that watches an element for resize — the rail's selection indicator does, to
 * re-measure when the rail is dragged or the arrow list changes height — takes
 * every test that mounts the rail down with it, before a single assertion runs.
 *
 * The stub deliberately observes nothing and never fires. jsdom performs no
 * layout at all: every element it renders is 0 × 0 and no box ever changes
 * size, so there is no resize for a faithful fake to report. One that invented
 * callbacks would be making up geometry the tests could then assert against
 * instead of asserting against the component, and the numbers would be fiction
 * either way — the indicator's real arithmetic is covered by
 * `indicator-frame.test.ts`, which takes rects as arguments precisely so it
 * does not need a layout engine.
 *
 * Defined rather than filled in conditionally, so the day jsdom ships a real
 * implementation is not the day the suite starts depending on one.
 */
class NoopResizeObserver implements ResizeObserver {
	observe(): void {}
	unobserve(): void {}
	disconnect(): void {}
}

Object.defineProperty(globalThis, 'ResizeObserver', {
	value: NoopResizeObserver,
	configurable: true,
	writable: true,
});
