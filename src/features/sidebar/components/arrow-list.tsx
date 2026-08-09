import { useMemo, type JSX } from 'react';

import { ScrollArea } from '@/components/ui/scroll-area';

import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowRow } from './arrow-row';

/**
 * The library, and the only part of the rail that scrolls.
 *
 * Give the scroll to the rail instead and the top bar and the three
 * destinations leave the screen as soon as the library is longer than the
 * window — including the back button, which is then the one control you cannot
 * reach.
 *
 * `ScrollArea` rather than a hand-rolled `overflow-y-auto` with
 * `[&::-webkit-scrollbar]` rules: those pseudo-elements are ignored the moment
 * anything sets `scrollbar-width`, which is a footgun a future stylesheet can
 * trip without touching this file. Base UI draws its own bar and behaves the
 * same in all three of the webviews this ships in.
 *
 * `overscrollContain` so a flick at the end of the library does not bounce the
 * window itself — on macOS that reads as the whole app coming loose.
 */
export function ArrowList(): JSX.Element {
	const { t, locale } = useTranslation();
	const arrows = useArrowStore((state) => state.arrows);

	// `localeCompare` under the active locale, not a raw `<`: the store is keyed
	// by namespace and the rail is read by name, and codepoint order puts
	// "Ångström" after "Zulu" for a Swedish reader who expects the opposite.
	const sorted = useMemo(
		() => [...arrows.values()].sort((a, b) => a.name.localeCompare(b.name, locale)),
		[arrows, locale]
	);

	return (
		<ScrollArea data-slot="scroll-area" overscrollContain className="min-h-0 flex-1">
			<nav aria-label={t('sidebar.arrows')} className="flex flex-col pb-1">
				{sorted.map((arrow) => (
					<ArrowRow key={arrow.namespace} arrow={arrow} />
				))}
			</nav>
		</ScrollArea>
	);
}
