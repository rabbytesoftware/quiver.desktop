import type { JSX } from 'react';

import type { CollectionArrow } from '@/domain/collection';
import { collectionArrowRoute } from '@/domain/collection';

import { CollectionArrowTile } from './collection-arrow-tile';
import '../styles/collection.css';

interface CollectionArrowGridProps {
	arrows: CollectionArrow[];
}

/** Unresolved arrows never render as a tile here -- they surface only in UnresolvedArrowsDialog, as bare routes. */
export function CollectionArrowGrid({ arrows }: CollectionArrowGridProps): JSX.Element {
	const resolved = arrows.filter((arrow) => arrow.resolved);
	return (
		<div className="collection-member-grid">
			{resolved.map((arrow) => (
				// The full route, not the bare namespace: two resolved members can
				// share a namespace at different versions (`owner/repo@v1` and
				// `owner/repo@v2`), and toCollectionArrow already split the version
				// into its own field before this ever sees it.
				<CollectionArrowTile key={collectionArrowRoute(arrow)} arrow={arrow} />
			))}
		</div>
	);
}
