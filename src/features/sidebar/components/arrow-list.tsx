import { useMemo, type JSX } from 'react';

import { ScrollArea } from '@/components/ui/scroll-area';

import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowRow } from './arrow-row';

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
		// `min-h-0` because the rail is a flex column: without it this pane's
		// content sets its floor and the list pushes the rail past the window
		// instead of scrolling inside it.
		<ScrollArea className="min-h-0 flex-1">
			<nav aria-label={t('sidebar.arrows')} className="flex flex-col">
				{sorted.map((arrow) => (
					<ArrowRow key={arrow.namespace} arrow={arrow} />
				))}
			</nav>
		</ScrollArea>
	);
}
