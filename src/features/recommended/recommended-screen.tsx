import { useMemo, type JSX } from 'react';

import { ArrowLeftIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { ShelfSection } from '@/features/home/components/shelf-section';
import { ShelfSkeleton } from '@/features/home/components/shelf-skeleton';
import { isSnapshotEmpty, visibleShelves } from '@/features/home/lib/visible-shelves';
import { useHome } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

/** The full view "View all" on Home's lead shelf leads to. */
export function RecommendedScreen(): JSX.Element {
	const { t } = useTranslation();
	const { data: home, isLoading } = useHome();

	const shelves = useMemo(() => visibleShelves(home), [home]);
	const total = useMemo(() => shelves.reduce((sum, shelf) => sum + shelf.arrows.length, 0), [shelves]);
	const showSkeleton = isSnapshotEmpty(home) && home?.refreshing === true;

	return (
		<div className="mx-auto w-full max-w-[1280px] px-6 pt-2 pb-6">
			<div className="mb-4">
				<Link
					className="inline-flex items-center gap-1 text-[12px] text-muted-foreground hover:text-foreground"
					to="/"
				>
					<ArrowLeftIcon aria-hidden="true" size={14} />
					{t('nav.home')}
				</Link>
				<div className="mt-1.5">
					<h1 className="text-[24px]/[28px] font-semibold tracking-[-0.4px]">{t('home.recommended')}</h1>
					{total > 0 && (
						<p className="mt-1 text-[12.5px] text-muted-foreground">
							{t('recommended.subtitle', { count: total })}
						</p>
					)}
				</div>
			</div>
			{shelves.map((shelf) => (
				<ShelfSection key={shelf.id} shelf={shelf} />
			))}
			{showSkeleton && <ShelfSkeleton title={t('home.recommended')} />}
			{!isLoading && shelves.length === 0 && !showSkeleton && (
				<p className="text-[13px] text-muted-foreground">{t('recommended.empty')}</p>
			)}
		</div>
	);
}
