import { useMemo, type JSX, type ReactNode } from 'react';

import { Link } from '@tanstack/react-router';

import type { ArrowEntry } from '@/domain/arrow';
import { columnRule } from '@/features/search/lib/columns';
import { ArrowTile } from '@/features/sidebar/components/arrows/arrow-tile';
import { arrowTileStatus } from '@/features/sidebar/components/arrows/arrow-tile-status';
import { useArrowStore, useFollowedCollections, useStop } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { EmptyHomeState } from './components/empty-home-state';

const RECENTS_LIMIT = 3;
const LIBRARY_PREVIEW_LIMIT = 10;
const COLLECTIONS_PREVIEW_LIMIT = 4;

function byName(a: { name: string }, b: { name: string }): number {
	return a.name.localeCompare(b.name);
}

interface SectionHeaderProps {
	title: string;
	action?: ReactNode;
}

function SectionHeader({ title, action }: SectionHeaderProps): JSX.Element {
	return (
		<div className="mb-4 flex items-end justify-between border-b border-border pb-2.5">
			<h2 className="text-[13px] font-semibold tracking-[-0.1px]">{title}</h2>
			{action}
		</div>
	);
}

function ViewAllLink({ to, children }: { to: '/library' | '/collections'; children: ReactNode }): JSX.Element {
	return (
		<Link
			className="inline-flex items-center gap-1 pb-2.5 text-[12px] text-muted-foreground hover:text-foreground"
			to={to}
		>
			{children}
		</Link>
	);
}

export function HomeScreen(): JSX.Element {
	const { t } = useTranslation();
	const arrows = useArrowStore((s) => s.arrows);
	const catalogStatus = useArrowStore((s) => s.catalog);
	const { data: collections = [], isLoading: collectionsLoading } = useFollowedCollections();
	const stop = useStop();

	const allArrows = useMemo(() => [...arrows.values()], [arrows]);

	const recents = useMemo(
		() =>
			allArrows
				.filter((a): a is ArrowEntry & { last_used_at: string } => Boolean(a.last_used_at))
				.sort((a, b) => (a.last_used_at < b.last_used_at ? 1 : -1))
				.slice(0, RECENTS_LIMIT),
		[allArrows]
	);

	const libraryPreview = useMemo(() => [...allArrows].sort(byName).slice(0, LIBRARY_PREVIEW_LIMIT), [allArrows]);

	const collectionsPreview = useMemo(
		() => [...collections].sort(byName).slice(0, COLLECTIONS_PREVIEW_LIMIT),
		[collections]
	);

	const isEmpty =
		catalogStatus !== 'loading' && !collectionsLoading && allArrows.length === 0 && collections.length === 0;
	if (isEmpty) return <EmptyHomeState />;

	return (
		<div className="mx-auto w-full max-w-[1120px] px-6 py-6">
			{recents.length > 0 && (
				<section className="mb-8">
					<SectionHeader title={t('home.recents')} />
					<div
						className="grid gap-x-3 gap-y-[18px]"
						style={{ gridTemplateColumns: 'repeat(3, minmax(0,1fr))' }}
					>
						{recents.map((arrow) => (
							<ArrowTile
								banner={arrow.banner}
								icon={arrow.icon}
								key={arrow.namespace}
								metaText={arrow.version}
								namespace={arrow.namespace}
								onResolve={() => stop.mutate({ namespace: arrow.namespace })}
								status={arrowTileStatus(arrow)}
								subtitle={arrow.description}
								title={arrow.name}
								to="/arrow/$"
							/>
						))}
					</div>
				</section>
			)}

			{allArrows.length > 0 && (
				<section className="mb-8">
					<SectionHeader
						action={
							<ViewAllLink to="/library">
								{t('home.viewAllArrows', { count: allArrows.length })}
							</ViewAllLink>
						}
						title={t('home.library')}
					/>
					<div
						className="grid gap-x-3 gap-y-[18px]"
						style={{ gridTemplateColumns: columnRule(libraryPreview.length) }}
					>
						{libraryPreview.map((arrow) => (
							<ArrowTile
								banner={arrow.banner}
								icon={arrow.icon}
								key={arrow.namespace}
								metaText={arrow.version}
								namespace={arrow.namespace}
								onResolve={() => stop.mutate({ namespace: arrow.namespace })}
								status={arrowTileStatus(arrow)}
								subtitle={arrow.description}
								title={arrow.name}
								to="/arrow/$"
							/>
						))}
					</div>
				</section>
			)}

			{collections.length > 0 && (
				<section className="mb-6">
					<SectionHeader
						action={
							<ViewAllLink to="/collections">
								{t('home.viewAllCollections', { count: collections.length })}
							</ViewAllLink>
						}
						title={t('home.collections')}
					/>
					<div
						className="grid gap-x-3 gap-y-[18px]"
						style={{ gridTemplateColumns: columnRule(collectionsPreview.length) }}
					>
						{collectionsPreview.map((collection) => (
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
				</section>
			)}
		</div>
	);
}
