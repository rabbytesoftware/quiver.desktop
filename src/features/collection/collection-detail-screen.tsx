import { useState, type JSX } from 'react';

import { useCollectionDetail } from '@/lib/core-store/queries/collection';

import { CollectionArrowGrid } from './components/collection-arrow-grid';
import { CollectionHero } from './components/collection-hero';
import { UnresolvedArrowsDialog } from './components/unresolved-arrows-dialog';
import './styles/collection.css';

interface CollectionDetailScreenProps {
	namespace: string;
}

export function CollectionDetailScreen({ namespace }: CollectionDetailScreenProps): JSX.Element {
	const { data, isLoading, isError } = useCollectionDetail(namespace);
	const [unresolvedOpen, setUnresolvedOpen] = useState(false);

	if (isLoading) return <div className="collection-loading">Loading...</div>;
	if (isError || !data) return <div className="collection-error">Couldn't load this collection.</div>;

	// toCollectionArrow (dtos/v0/collection.ts) already splits the version off
	// `namespace` on every member regardless of resolved status -- an
	// unresolved arrow's wire namespace is still `owner/repo@version`, so
	// `arrow.version` is populated here too. Reattach it: the dialog is
	// supposed to show the exact route that was asked for
	// (github.com/rabbyte/ark-survival@v3.1.0), not the bare package path.
	const unresolvedRoutes = data.arrows
		.filter((arrow) => !arrow.resolved)
		.map((arrow) => (arrow.version ? `${arrow.namespace}@${arrow.version}` : arrow.namespace));

	return (
		<div className="collection-detail-screen">
			<CollectionHero
				collection={data}
				unresolvedCount={unresolvedRoutes.length}
				onUnresolvedClick={() => setUnresolvedOpen(true)}
			/>
			<div className="collection-section-head">
				<span className="collection-section-title">Arrows</span>
			</div>
			<div className="collection-panel">
				<CollectionArrowGrid arrows={data.arrows} />
			</div>
			<UnresolvedArrowsDialog open={unresolvedOpen} onOpenChange={setUnresolvedOpen} routes={unresolvedRoutes} />
		</div>
	);
}
