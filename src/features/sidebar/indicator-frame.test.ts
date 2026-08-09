import { describe, expect, it } from 'vitest';

import { indicatorFrame, scrolledY, type Box } from './indicator-frame';

/** The rail, offset from the viewport so a bug that forgets to subtract shows. */
const RAIL: Box = { top: 40, left: 12, width: 246, height: 600 };

/** The scroller under the nav: 500px tall, starting 100px down the rail. */
const LIST: Box = { top: 140, left: 12, width: 246, height: 500 };

function row(top: number): Box {
	return { top, left: 18, width: 234, height: 36 };
}

describe('indicatorFrame', () => {
	it('measures against the rail, not the viewport', () => {
		const frame = indicatorFrame(row(200), RAIL, LIST);

		expect(frame.x).toBe(6);
		expect(frame.y).toBe(160);
		expect(frame.width).toBe(234);
		expect(frame.height).toBe(36);
	});

	it('never clips a nav segment', () => {
		// `null` says the target is not in the scroller at all — a nav segment
		// sits above it and has no band to stay inside.
		const frame = indicatorFrame({ top: 60, left: 20, width: 70, height: 36 }, RAIL, null);

		expect(frame.clipTop).toBe(0);
		expect(frame.clipBottom).toBe(0);
		expect(frame.visible).toBe(true);
	});

	it('leaves a fully visible row unclipped', () => {
		const frame = indicatorFrame(row(300), RAIL, LIST);

		expect(frame.clipTop).toBe(0);
		expect(frame.clipBottom).toBe(0);
		expect(frame.visible).toBe(true);
	});

	/**
	 * The case that would otherwise paint the highlight over the nav. A row
	 * scrolled halfway under the list's top edge keeps the visible half.
	 */
	it('clips the overhang past the list’s top edge', () => {
		const frame = indicatorFrame(row(125), RAIL, LIST);

		expect(frame.clipTop).toBe(15);
		expect(frame.clipBottom).toBe(0);
		expect(frame.visible).toBe(true);
	});

	it('clips the overhang past the list’s bottom edge', () => {
		// Band ends at rail-y 600 (140 - 40 + 500); this row runs to 610.
		const frame = indicatorFrame(row(614), RAIL, LIST);

		expect(frame.clipTop).toBe(0);
		expect(frame.clipBottom).toBe(10);
		expect(frame.visible).toBe(true);
	});

	it('goes invisible once nothing of the row is left', () => {
		expect(indicatorFrame(row(60), RAIL, LIST).visible).toBe(false);
		expect(indicatorFrame(row(700), RAIL, LIST).visible).toBe(false);
	});

	/**
	 * A row resting exactly on the boundary has zero visible pixels. Painting a
	 * zero-height sliver costs a composite for something nobody can see, so the
	 * comparison is `<`, not `<=`.
	 */
	it('treats a row flush with the edge as gone, not as a sliver', () => {
		// Runs to exactly the band's top (rail-y 100).
		const flush = indicatorFrame(row(104), RAIL, LIST);

		expect(flush.clipTop).toBe(36);
		expect(flush.visible).toBe(false);
	});
});

describe('scrolledY', () => {
	it('is the identity at the scroll position it was measured against', () => {
		expect(scrolledY(160, 250, 250)).toBe(160);
	});

	it('moves the indicator up as the list scrolls down', () => {
		expect(scrolledY(160, 250, 300)).toBe(110);
	});

	it('moves it down as the list scrolls back', () => {
		expect(scrolledY(160, 250, 200)).toBe(210);
	});

	/**
	 * Measuring while already scrolled is the normal case — you click a row 80
	 * rows down. The delta has to be against the scroll that produced `baseY`,
	 * not against zero, or the indicator lands a screenful away.
	 */
	it('is relative to the scroll it was captured at, not to zero', () => {
		expect(scrolledY(200, 1_000, 1_000)).toBe(200);
		expect(scrolledY(200, 1_000, 1_036)).toBe(164);
	});
});
