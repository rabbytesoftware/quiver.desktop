import { useCallback, useEffect, useLayoutEffect, useRef, type JSX } from 'react';

import { useVirtualizer } from '@tanstack/react-virtual';

import { useTranslation } from '@/lib/i18n';

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
	/** Changes when something the user did (running a command) should bring them back to the newest line. */
	tailKey: number;
}

/** How long after the user touches the pane a scroll still counts as theirs. */
const USER_SCROLL_WINDOW_MS = 250;

/**
 * The scrolling log. Virtualized, because the buffer holds thousands of lines
 * and a line's height depends on how its fields wrap.
 *
 * It sticks to the newest line until the user scrolls up, and resumes when they
 * come back down. The rule that makes that reliable is who decides "the user
 * left": only a scroll that follows the user's own input (wheel, touch, pointer on
 * the scrollbar, keys) can stop it. A scroll event on its own proves nothing --
 * rows are laid out at an estimated height and then measured, the buffer trims
 * its oldest lines from the top, and the browser clamps the position when the
 * content changes, so scroll events arrive from layout that no one asked for, at
 * distances from the bottom that mean nothing. Judging by those left the pane on
 * the oldest lines of a busy daemon in WebKitGTK, with the newest never rendered.
 *
 * While following, the pane is pinned (`scrollTop = scrollHeight`) whenever the
 * lines or the measured total height change, once at once and once a frame
 * later, after the browser has laid the new rows out. Running a command brings
 * the pane back to the newest line, however far up it was left.
 */
export function ConsoleLog({ entries, expandedId, onToggle, revealKey, tailKey }: ConsoleLogProps): JSX.Element {
	const { t } = useTranslation();
	const scroller = useRef<HTMLDivElement>(null);
	const following = useRef(true);
	const userActive = useRef(false);
	const userTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

	const virtualizer = useVirtualizer({
		count: entries.length,
		getScrollElement: () => scroller.current,
		estimateSize: () => ESTIMATED_ROW,
		overscan: 15,
		getItemKey: (index) => entries[index].id,
	});
	// The pane pins itself; the virtualizer shifting the position as rows are
	// measured would fight it. (An instance property, not an option.)
	virtualizer.shouldAdjustScrollPositionOnItemSizeChange = () => false;
	const total = virtualizer.getTotalSize();

	const markUser = useCallback(() => {
		userActive.current = true;
		if (userTimer.current !== null) clearTimeout(userTimer.current);
		userTimer.current = setTimeout(() => {
			userActive.current = false;
		}, USER_SCROLL_WINDOW_MS);
	}, []);

	// The user's own input, listened for on the element itself: these are not
	// handlers for the pane to act on, only the evidence that a scroll is the user's.
	useEffect(() => {
		const el = scroller.current;
		if (!el) return;
		// A scrollable region the keyboard can reach, so the arrow and page keys scroll it.
		el.tabIndex = 0;
		// Only the scrollbar and the gutter press the pane itself: a click on a row is not a scroll.
		const press = (event: Event): void => {
			if (event.target === el) markUser();
		};
		const options = { passive: true } as const;
		el.addEventListener('wheel', markUser, options);
		el.addEventListener('touchmove', markUser, options);
		el.addEventListener('keydown', markUser);
		el.addEventListener('pointerdown', press);
		return () => {
			el.removeEventListener('wheel', markUser);
			el.removeEventListener('touchmove', markUser);
			el.removeEventListener('keydown', markUser);
			el.removeEventListener('pointerdown', press);
			if (userTimer.current !== null) clearTimeout(userTimer.current);
		};
	}, [markUser]);

	const onScroll = useCallback(() => {
		const el = scroller.current;
		if (!el || !userActive.current) return;
		following.current = el.scrollHeight - el.scrollTop - el.clientHeight < FOLLOW_SLACK;
	}, []);

	useLayoutEffect(() => {
		following.current = true;
	}, [revealKey, tailKey]);

	useLayoutEffect(() => {
		const el = scroller.current;
		if (!el || !following.current || entries.length === 0) return;
		const pin = (): void => {
			if (following.current) el.scrollTop = el.scrollHeight;
		};
		pin();
		const frame = requestAnimationFrame(pin);
		return () => cancelAnimationFrame(frame);
	}, [entries.length, total, expandedId, revealKey, tailKey]);

	return (
		<div
			ref={scroller}
			onScroll={onScroll}
			data-slot="console-log"
			role="log"
			aria-live="off"
			aria-label={t('console.log.label')}
			className="min-h-0 flex-1 overflow-y-auto py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
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
