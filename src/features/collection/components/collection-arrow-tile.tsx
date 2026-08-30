import type { CSSProperties, JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { CollectionArrow } from '@/domain/collection';
import { collectionArrowRoute } from '@/domain/collection';
import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { ownerOf } from '@/lib/namespace';

import '@/features/search/styles/card.css';
import '../styles/collection.css';

interface CollectionArrowTileProps {
	arrow: CollectionArrow;
}

/** A resolved member always carries a name (quiver.core sets it in the same branch as `resolved: true`); this is a display floor, not a real fallback path. */
function displayName(arrow: CollectionArrow): string {
	if (arrow.name) return arrow.name;
	const parts = arrow.namespace.split('/');
	return parts[parts.length - 1] || arrow.namespace;
}

/**
 * Same shape as ArrowCard (arrow-card.tsx), reusing card.css's real
 * data-slot selectors for the verbatim spring transform -- data-slot="arrow-card"
 * on this root is what `[data-slot='arrow-card']:hover [data-slot='card-banner']`
 * actually hooks the hover kick to, and is a real `<Link>` to `/arrow/$` for the
 * same reason ArrowCard is: it looks and hovers like the real thing, so it
 * navigates like it too. A collection member has no icon or banner field at all
 * (CollectionArrowDTO carries namespace/resolved/name/description only), so the
 * drawn-ghost fallback is the only state this ever renders -- not a rare edge
 * case here the way it is for a real, catalog-backed ArrowCard.
 */
export function CollectionArrowTile({ arrow }: CollectionArrowTileProps): JSX.Element {
	const name = displayName(arrow);

	return (
		<Link
			className="collection-member-cell"
			data-slot="arrow-card"
			params={{ _splat: collectionArrowRoute(arrow) }}
			to="/arrow/$"
		>
			<span className="collection-member-frame">
				{/* absolute inset-0 overflow-hidden rounded-lg bg-muted bg-cover
				    bg-center: arrow-card.tsx's own inline classes for this slot --
				    card.css only supplies the transform/transition, not the box
				    model, so this copies arrow-card.tsx verbatim rather than
				    assuming importing the stylesheet was enough. */}
				<span
					className="absolute inset-0 overflow-hidden rounded-lg bg-muted bg-cover bg-center"
					data-slot="card-banner"
				>
					<span data-slot="card-drawn">
						<span data-slot="drawn-ghost">{name.slice(0, 1).toUpperCase()}</span>
						<span data-slot="drawn-type">
							<span data-slot="drawn-name">{name}</span>
							<span data-slot="drawn-owner">{ownerOf(arrow.namespace)}</span>
						</span>
					</span>
				</span>
				<span className="collection-member-info" data-slot="card-info">
					<span className="collection-member-info-icon-wrap" style={{ '--icon': '20px' } as CSSProperties}>
						<ArrowIcon namespace={arrow.namespace} name={name} icon={null} />
					</span>
					{arrow.version && <span className="collection-member-info-text">{arrow.version}</span>}
				</span>
			</span>
			<span className="collection-member-caption">
				<span className="collection-member-caption-name">{name}</span>
				<span className="collection-member-caption-sub">{arrow.namespace}</span>
			</span>
		</Link>
	);
}
