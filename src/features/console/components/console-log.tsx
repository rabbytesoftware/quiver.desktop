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
 * and a line's height depends on how its fields wrap. It follows the newest line
 * until the user scrolls away from the bottom, and resumes when they return.
 */
export function ConsoleLog({ entries, expandedId, onToggle, revealKey }: ConsoleLogProps): JSX.Element {
	const scroller = useRef<HTMLDivElement>(null);
	const following = useRef(true);

	const virtualizer = useVirtualizer({
		count: entries.length,
		getScrollElement: () => scroller.current,
		estimateSize: () => ESTIMATED_ROW,
		overscan: 15,
		getItemKey: (index) => entries[index].id,
	});

	const onScroll = useCallback(() => {
		const el = scroller.current;
		if (!el) return;
		following.current = el.scrollHeight - el.scrollTop - el.clientHeight < FOLLOW_SLACK;
	}, []);

	useLayoutEffect(() => {
		following.current = true;
	}, [revealKey]);

	useLayoutEffect(() => {
		if (following.current && entries.length > 0) virtualizer.scrollToIndex(entries.length - 1, { align: 'end' });
	}, [entries.length, expandedId, revealKey, virtualizer]);

	return (
		<div
			ref={scroller}
			onScroll={onScroll}
			data-slot="console-log"
			role="log"
			aria-live="off"
			className="min-h-0 flex-1 overflow-y-auto py-2"
		>
			<div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
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
