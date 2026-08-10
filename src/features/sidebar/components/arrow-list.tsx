import { useMemo, type JSX } from 'react';

import { ScrollArea } from '@/components/ui/scroll-area';

import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowRow } from './arrow-row';

export function ArrowList(): JSX.Element {
	const { t, locale } = useTranslation();
	const arrows = useArrowStore((state) => state.arrows);

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
