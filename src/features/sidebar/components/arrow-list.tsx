import { useMemo, type JSX } from 'react';

import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowRow } from './arrow-row';

/**
 * The design styles the scrollbar itself — 6px wide, thumb `--border`, no track
 * and no buttons — which is a native `::-webkit-scrollbar` and not what shadcn's
 * ScrollArea draws. That component hides the real bar and renders its own from
 * measurements it takes of the viewport, and the rail's width is dragged live:
 * it becomes one more thing that has to be told the layout moved, in exchange
 * for a scrollbar the design already describes in three declarations.
 *
 * `min-h-0` because the rail is a flex column: without it this pane's content
 * sets its floor and the list pushes the rail past the window instead of
 * scrolling inside it. `overflow-x-hidden` because the rows are `--rail` wide
 * with no wrap, and a sub-pixel rounding at some rail widths is enough to hand
 * the column a horizontal bar it can never usefully scroll.
 *
 * `data-slot="scroll-area"` outlives the component it came from: it is how the
 * rail's own test says the list scrolls and the top bar above it does not.
 */
const SCROLLER = [
	'min-h-0 flex-1 overflow-x-hidden overflow-y-auto',
	'[&::-webkit-scrollbar]:w-[6px] [&::-webkit-scrollbar-thumb]:bg-border',
].join(' ');

/**
 * Every installed arrow, in name order.
 *
 * The store hands back a `Map`, whose order is insertion order — which is to
 * say whatever the catalog stream happened to do, and it reseeds on every
 * reconnect. Rendering it unsorted lets the list reorder itself under the
 * cursor while nothing the user did changed.
 *
 * `localeCompare` with the active locale, not `<`: `<` compares UTF-16 code
 * units, which sorts every capital ahead of every lowercase letter and files
 * accented names after `Z`.
 */
export function ArrowList(): JSX.Element {
	const { t, locale } = useTranslation();
	const arrows = useArrowStore((state) => state.arrows);

	const sorted = useMemo(
		() => [...arrows.values()].sort((a, b) => a.name.localeCompare(b.name, locale)),
		[arrows, locale]
	);

	return (
		<div data-slot="scroll-area" className={SCROLLER}>
			<nav aria-label={t('sidebar.arrows')} className="flex flex-col">
				{sorted.map((arrow) => (
					<ArrowRow key={arrow.namespace} arrow={arrow} />
				))}
			</nav>
		</div>
	);
}
