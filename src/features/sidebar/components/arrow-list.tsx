import { useMemo, type JSX } from 'react';

import { ScrollArea } from '@/components/ui/scroll-area';

import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowListNotice } from './arrow-list-notice';
import { ArrowListSkeleton } from './arrow-list-skeleton';
import { ArrowRow } from './arrow-row';

export function ArrowList(): JSX.Element {
	const { t, locale } = useTranslation();
	const arrows = useArrowStore((state) => state.arrows);
	const catalog = useArrowStore((state) => state.catalog);

	const sorted = useMemo(
		() => [...arrows.values()].sort((a, b) => a.name.localeCompare(b.name, locale)),
		[arrows, locale]
	);

	return (
		<ScrollArea data-slot="scroll-area" overscrollContain className="min-h-0 flex-1">
			<nav aria-label={t('sidebar.arrows')} className="flex flex-col pb-1">
				{sorted.length > 0 ? (
					sorted.map((arrow) => <ArrowRow arrow={arrow} key={arrow.namespace} />)
				) : catalog === 'error' ? (
					<ArrowListNotice
						action={{
							label: t('sidebar.arrows.error.action'),
							to: '/settings',
							search: { tab: 'engine' },
						}}
						body={t('sidebar.arrows.error.body')}
						title={t('sidebar.arrows.error.title')}
					/>
				) : catalog === 'loading' ? (
					<ArrowListSkeleton label={t('sidebar.arrows.loading')} />
				) : (
					<ArrowListNotice body={t('sidebar.arrows.empty.body')} title={t('sidebar.arrows.empty.title')} />
				)}
			</nav>
		</ScrollArea>
	);
}
