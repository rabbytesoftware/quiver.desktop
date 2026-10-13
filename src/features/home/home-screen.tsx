import { useMemo, type JSX } from 'react';

import type { ArrowEntry } from '@/domain/arrow';
import { isInferred } from '@/domain/arrow';
import { isQuiverOwnComponent } from '@/domain/release';
import { columnRule } from '@/features/search/lib/columns';
import { useCatalogVisibilityStore } from '@/features/settings/stores/catalog-visibility-store';
import { ArrowTile } from '@/features/sidebar/components/arrows/arrow-tile';
import { arrowTileStatus } from '@/features/sidebar/components/arrows/arrow-tile-status';
import { useArrowStore, useFollowedCollections, useHome, useStop } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { EmptyHomeState } from './components/empty-home-state';
import { SectionHeader } from './components/section-header';
import { ShelfSection } from './components/shelf-section';
import { ShelfSkeleton } from './components/shelf-skeleton';
import { ViewAllLink } from './components/view-all-link';
import { isSnapshotEmpty, visibleShelves } from './lib/visible-shelves';

const RECENTS_LIMIT = 3;
const LIBRARY_PREVIEW_LIMIT = 10;
const COLLECTIONS_PREVIEW_LIMIT = 4;
const LEAD_SHELF_LIMIT = 3;

function byName(a: { name: string }, b: { name: string }): number {
	return a.name.localeCompare(b.name);
}

export function HomeScreen(): JSX.Element {
	const { t } = useTranslation();
	const arrows = useArrowStore((s) => s.arrows);
	const catalogStatus = useArrowStore((s) => s.catalog);
	const { data: collections = [], isLoading: collectionsLoading } = useFollowedCollections();
	const { data: home, isLoading: homeLoading } = useHome();
	const stop = useStop();
	const showSelfComponents = useCatalogVisibilityStore((s) => s.showSelfComponents);

	const allArrows = useMemo(
		() => [...arrows.values()].filter((a) => showSelfComponents || !isQuiverOwnComponent(a.namespace)),
		[arrows, showSelfComponents]
	);

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

	const shelves = useMemo(() => visibleShelves(home), [home]);

	const [leadShelf, ...trailingShelves] = shelves;

	const showSkeleton = isSnapshotEmpty(home) && home?.refreshing === true;

	const isEmpty =
		catalogStatus !== 'loading' &&
		!collectionsLoading &&
		!homeLoading &&
		allArrows.length === 0 &&
		collections.length === 0 &&
		shelves.length === 0 &&
		!showSkeleton;
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
								confidence={arrow.confidence}
								icon={arrow.icon}
								inferred={isInferred(arrow)}
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

			{leadShelf && (
				<ShelfSection
					action={
						leadShelf.arrows.length > LEAD_SHELF_LIMIT && (
							<ViewAllLink to="/recommended">
								{t('home.viewAllRecommended', { count: leadShelf.arrows.length })}
							</ViewAllLink>
						)
					}
					shelf={{ ...leadShelf, arrows: leadShelf.arrows.slice(0, LEAD_SHELF_LIMIT) }}
				/>
			)}

			{showSkeleton && <ShelfSkeleton title={t('home.recommended')} />}

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
								confidence={arrow.confidence}
								icon={arrow.icon}
								inferred={isInferred(arrow)}
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

			{trailingShelves.map((shelf) => (
				<ShelfSection key={shelf.id} shelf={shelf} />
			))}

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
