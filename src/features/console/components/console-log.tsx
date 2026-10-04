import { useCallback, useLayoutEffect, useRef, type JSX } from 'react';

import { useVirtualizer } from '@tanstack/react-virtual';

import { EntryRow } from './entry-row';
import type { ConsoleEntry } from '../stores/console-store';

/** A rough row height; each row is measured once it has rendered. */
const ESTIMATED_ROW = 20;

/** Within this many pixels of the bottom, the view follows new lines. */
const FOLLOW_SLACK = 32;

export interface ConsoleLogProps {
	entries: readonly ConsoleEntry[];
	expandedId: number | null;
	onToggle: (id: number) => void;
	/** Changes when the console is shown, so the view returns to the newest line. */
	revealKey: unknown;
}

/**
 * The scrolling log. Virtualized, because the buffer holds thousands of lines
 * and a line's height depends on how its fields wrap.
 *
 * It sticks to the bottom -- the newest line -- until the user scrolls up, and
 * resumes when they come back down. How it decides is the delicate part:
 *
 *  - It pins by setting `scrollTop` to `scrollHeight`, not by asking the
 *    virtualizer to scroll to the last index. Rows are first laid out at an
 *    estimated height and then measured, so the content keeps growing under a
 *    view that is already at the bottom; an index-based scroll computed from the
 *    estimates stops short, leaves the window on rows far from the end, and the
 *    newest lines are never rendered. Re-pinning every time the total height
 *    changes follows the measured size instead.
 *  - It leaves the bottom only when the user moves UP. A scroll event whose
 *    position did not decrease is the content growing (or our own pinning), not
 *    the user, however far from the bottom the stale geometry says it is. Judging
 *    by distance alone un-pins on the first measurement after a burst of lines,
 *    which is exactly when a busy daemon is writing them.
 */
export function ConsoleLog({ entries, expandedId, onToggle, revealKey }: ConsoleLogProps): JSX.Element {
	const scroller = useRef<HTMLDivElement>(null);
	const following = useRef(true);
	const lastTop = useRef(0);

	const virtualizer = useVirtualizer({
		count: entries.length,
		getScrollElement: () => scroller.current,
		estimateSize: () => ESTIMATED_ROW,
		overscan: 15,
		getItemKey: (index) => entries[index].id,
	});
	const total = virtualizer.getTotalSize();

	const onScroll = useCallback(() => {
		const el = scroller.current;
		if (!el) return;
		const top = el.scrollTop;
		if (el.scrollHeight - top - el.clientHeight < FOLLOW_SLACK) following.current = true;
		else if (top < lastTop.current) following.current = false;
		lastTop.current = top;
	}, []);

	useLayoutEffect(() => {
		following.current = true;
	}, [revealKey]);

	useLayoutEffect(() => {
		const el = scroller.current;
		if (!el || !following.current || entries.length === 0) return;
		el.scrollTop = el.scrollHeight;
		lastTop.current = el.scrollTop;
	}, [entries.length, total, expandedId, revealKey]);

	return (
		<div
			ref={scroller}
			onScroll={onScroll}
			data-slot="console-log"
			role="log"
			aria-live="off"
			className="min-h-0 flex-1 overflow-y-auto py-2"
		>
			<div className="relative w-full" style={{ height: total }}>
				{virtualizer.getVirtualItems().map((item) => (
					<div
						key={item.key}
						data-index={item.index}
						ref={virtualizer.measureElement}
						className="absolute top-0 left-0 w-full"
						style={{ transform: `translateY(${item.start}px)` }}
					>
						<EntryRow
							entry={entries[item.index]}
							expanded={entries[item.index].id === expandedId}
							onToggle={onToggle}
						/>
					</div>
				))}
			</div>
		</div>
	);
}
