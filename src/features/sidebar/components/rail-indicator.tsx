import { useEffect, useRef, type JSX } from 'react';

import { useRouterState } from '@tanstack/react-router';

import { indicatorFrame, scrolledY } from '../indicator-frame';
import { RAIL_INDICATOR } from '../row-base';

/**
 * How the rail marks where you are: one filled box that travels to whatever was
 * clicked — between the three nav segments, and between those and any arrow row.
 *
 * A SINGLE element for the whole rail, which is the only way the movement can be
 * continuous across those two regions. Base UI's `Tabs.Indicator` cannot do it:
 * it measures within its own list and knows nothing about a row four hundred
 * pixels below it in a different scroll container.
 *
 * ── finding the target ───────────────────────────────────────────────────
 * Every selectable surface in the rail is a router `<Link>`, and the router
 * already marks the current one `data-status="active"`. So the target is one
 * query — no registry, no context, no ids threaded through props, and no second
 * copy of the selection that could disagree with the URL (spec §5.1).
 *
 * ── staying cheap ────────────────────────────────────────────────────────
 * Measuring happens only when something can actually have moved: the selection
 * changed, the rail was dragged, or the list changed height. Never on a render,
 * never on a scroll.
 *
 * Scrolling replays arithmetic instead. `baseY` and `baseScroll` are captured
 * together at measure time and every scroll frame recomputes `y` from the delta,
 * so the handler never reads geometry. That matters more than it sounds: the
 * handler also WRITES a style, and a read-after-write across frames is exactly
 * what forces synchronous layout.
 *
 * The listener is attached only while a ROW is selected. With a nav segment
 * selected, or nothing, scrolling the library does no indicator work at all.
 */
export function RailIndicator(): JSX.Element {
	const element = useRef<HTMLDivElement>(null);
	/** Captured at measure time; the scroll path's only inputs. */
	const anchor = useRef({ x: 0, baseY: 0, baseScroll: 0, height: 0 });
	/**
	 * The last value written for each property, so a scroll frame can skip the
	 * ones that have not changed.
	 *
	 * This is not micro-optimisation, it is the difference between smooth and
	 * not. `transform` is composited and costs almost nothing to rewrite, but
	 * `clip-path` is a PAINT property: rewriting it re-rasterises the layer, and
	 * doing that every frame put p95 at 29ms against a 18ms control, with the
	 * worst frames past 50. During most of a scroll the clip does not change at
	 * all — the row is either fully inside the list or fully outside it — so the
	 * cache removes the cost for all but the handful of partial frames.
	 */
	const written = useRef({ transform: '', clip: '', visible: '' });

	// The pathname, not the whole location: a `?q=` keystroke on /search would
	// otherwise re-measure the rail on every character typed.
	const pathname = useRouterState({ select: (state) => state.location.pathname });

	useEffect(() => {
		const indicator = element.current;
		if (indicator === null) return;

		const rail = indicator.parentElement;
		if (rail === null) return;

		/**
		 * Writes only what differs. See `written` — skipping an unchanged
		 * `clip-path` is what keeps a scrolling frame off the paint path.
		 */
		const write = (transform: string, clip: string, visible: string): void => {
			const last = written.current;

			if (transform !== last.transform) {
				indicator.style.transform = transform;
				last.transform = transform;
			}
			if (clip !== last.clip) {
				indicator.style.clipPath = clip;
				last.clip = clip;
			}
			if (visible !== last.visible) {
				indicator.dataset.visible = visible;
				last.visible = visible;
			}
		};

		const target = rail.querySelector<HTMLElement>('a[data-status="active"]');

		// Nothing selected — /search, or any route the rail does not list. Fade
		// out, and attach nothing.
		if (target === null) {
			indicator.dataset.visible = 'false';
			return;
		}

		// `null` for a nav segment, which sits outside the scroller and so has no
		// band to be clipped against.
		const viewport = target.closest<HTMLElement>('[data-slot="scroll-area-viewport"]');

		const measure = (): void => {
			const frame = indicatorFrame(
				target.getBoundingClientRect(),
				rail.getBoundingClientRect(),
				viewport === null ? null : viewport.getBoundingClientRect()
			);

			anchor.current = {
				x: frame.x,
				baseY: frame.y,
				baseScroll: viewport?.scrollTop ?? 0,
				height: frame.height,
			};

			indicator.style.width = `${frame.width}px`;
			indicator.style.height = `${frame.height}px`;
			write(
				`translate3d(${frame.x}px, ${frame.y}px, 0)`,
				`inset(${frame.clipTop}px 0 ${frame.clipBottom}px 0)`,
				String(frame.visible)
			);
		};

		measure();

		// Released a frame later, once the browser has committed the position
		// above — otherwise the very first paint animates in from the corner.
		const settle = requestAnimationFrame(() => {
			indicator.dataset.ready = 'true';
		});

		// Only things that genuinely move the target: the rail being dragged, and
		// the list changing height (arrows arriving, a locale change re-sorting).
		const remeasure = new ResizeObserver(measure);
		remeasure.observe(rail);
		if (viewport !== null) remeasure.observe(viewport);

		if (viewport === null) {
			return () => {
				cancelAnimationFrame(settle);
				remeasure.disconnect();
			};
		}

		const scroller = viewport;
		let queued = 0;

		const follow = (): void => {
			queued = 0;

			const { x, baseY, baseScroll, height } = anchor.current;
			const y = scrolledY(baseY, baseScroll, scroller.scrollTop);

			// `offsetTop` and `clientHeight` are cached layout on an element
			// nothing has mutated this frame, so neither forces a reflow.
			const bandTop = scroller.offsetTop;
			const bandBottom = bandTop + scroller.clientHeight;
			const clipTop = Math.max(0, bandTop - y);
			const clipBottom = Math.max(0, y + height - bandBottom);

			write(
				`translate3d(${x}px, ${y}px, 0)`,
				`inset(${clipTop}px 0 ${clipBottom}px 0)`,
				String(clipTop + clipBottom < height)
			);
		};

		const onScroll = (): void => {
			indicator.dataset.scrolling = 'true';
			// Coalesced: a burst of scroll events collapses to one write a frame.
			if (queued === 0) queued = requestAnimationFrame(follow);
		};

		scroller.addEventListener('scroll', onScroll, { passive: true });

		return () => {
			cancelAnimationFrame(settle);
			if (queued !== 0) cancelAnimationFrame(queued);
			remeasure.disconnect();
			scroller.removeEventListener('scroll', onScroll);
			// Cleared here rather than on scroll end, so the next selection change
			// animates. Travel is the only time the transition is wanted.
			delete indicator.dataset.scrolling;
		};
	}, [pathname]);

	return (
		<div
			ref={element}
			data-slot="rail-indicator"
			aria-hidden="true"
			data-visible="false"
			className={RAIL_INDICATOR}
		/>
	);
}
