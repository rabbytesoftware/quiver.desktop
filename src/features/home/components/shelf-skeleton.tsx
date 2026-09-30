import type { JSX } from 'react';

import { CardSkeleton } from '@/features/search/components/card/card-skeleton';
import { columnRule } from '@/features/search/lib/columns';

import { SectionHeader } from './section-header';

const SKELETON_COUNT = 4;

export function ShelfSkeleton({ title }: { title: string }): JSX.Element {
	return (
		<section className="mb-8" data-slot="home-shelf-loading">
			<SectionHeader title={title} />
			<div className="grid gap-x-3 gap-y-[18px]" style={{ gridTemplateColumns: columnRule(SKELETON_COUNT) }}>
				<CardSkeleton count={SKELETON_COUNT} />
			</div>
		</section>
	);
}
