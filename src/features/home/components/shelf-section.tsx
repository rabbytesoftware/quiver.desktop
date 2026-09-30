import type { JSX, ReactNode } from 'react';

import { isInferred } from '@/domain/arrow';
import type { HomeShelf } from '@/domain/home';
import { columnRule } from '@/features/search/lib/columns';
import { ArrowTile } from '@/features/sidebar/components/arrows/arrow-tile';

import { SectionHeader } from './section-header';

interface ShelfSectionProps {
	shelf: HomeShelf;
	action?: ReactNode;
}

export function ShelfSection({ shelf, action }: ShelfSectionProps): JSX.Element {
	return (
		<section className="mb-8" data-slot="home-shelf">
			<SectionHeader action={action} title={shelf.title} />
			<div className="grid gap-x-3 gap-y-[18px]" style={{ gridTemplateColumns: columnRule(shelf.arrows.length) }}>
				{shelf.arrows.map((arrow) => (
					<ArrowTile
						banner={arrow.banner}
						confidence={arrow.confidence}
						icon={arrow.icon}
						inferred={isInferred(arrow)}
						key={arrow.namespace}
						metaText={arrow.versions[0] ?? ''}
						namespace={arrow.namespace}
						status={null}
						subtitle={arrow.description}
						title={arrow.name}
						to="/arrow/$"
					/>
				))}
			</div>
		</section>
	);
}
