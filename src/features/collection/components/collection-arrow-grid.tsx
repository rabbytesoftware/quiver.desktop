import type { JSX } from 'react';

import type { CollectionArrow } from '@/domain/collection';

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
				<CollectionArrowTile key={arrow.namespace} arrow={arrow} />
			))}
		</div>
	);
}
