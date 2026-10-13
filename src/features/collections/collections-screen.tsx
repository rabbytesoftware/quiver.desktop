import { useMemo, type JSX } from 'react';

import { ArrowLeftIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { columnRule } from '@/features/search/lib/columns';
import { ArrowTile } from '@/features/sidebar/components/arrows/arrow-tile';
import { useFollowedCollections } from '@/lib/core-store/queries/collections';
import { useTranslation } from '@/lib/i18n';

function byName(a: { name: string }, b: { name: string }): number {
	return a.name.localeCompare(b.name);
}

/** The full view "View all N collections" on Home leads to -- see docs/home-page-spec.md §5.4. */
export function CollectionsScreen(): JSX.Element {
	const { t } = useTranslation();
	const { data: collections = [] } = useFollowedCollections();

	const sorted = useMemo(() => [...collections].sort(byName), [collections]);

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
					<h1 className="text-[24px]/[28px] font-semibold tracking-[-0.4px]">{t('home.collections')}</h1>
					<p className="mt-1 text-[12.5px] text-muted-foreground">
						{t('collections.subtitle', { count: sorted.length })}
					</p>
				</div>
			</div>
			<div className="grid gap-x-3 gap-y-[18px]" style={{ gridTemplateColumns: columnRule(sorted.length) }}>
				{sorted.map((collection) => (
					<ArrowTile
						banner={null}
						icon={null}
						key={collection.namespace}
						metaText={t('collections.arrowCount', { count: collection.arrowCount })}
						namespace={collection.namespace}
						status={null}
						subtitle={collection.description}
						title={collection.name}
						to="/collection/$"
					/>
				))}
			</div>
		</div>
	);
}
