import type { JSX } from 'react';

import type { SearchEntry } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';

import { ArrowCard } from './arrow-card';
import { CardSkeleton } from './card-skeleton';

interface ResultGridProps {
	local: SearchEntry[];
	streamed: SearchEntry[];
	phase: SearchPhase;
}

const SKELETON_COUNT = 4;

/** pt-3 (12px) + the header's 18px (results-header.tsx) is --reveal (30px, spec 8.2/9.2). */
const GRID = 'grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-x-3 gap-y-3.5 px-[18px] pt-3 pb-[22px]';

export function ResultGrid({ local, streamed, phase }: ResultGridProps): JSX.Element {
	const showSeam = streamed.length > 0 && local.length > 0 && (phase === 'discovering' || phase === 'settling');

	return (
		<div className={GRID}>
			{streamed.map((entry) => (
				<ArrowCard entry={entry} key={entry.namespace} />
			))}
			{phase === 'discovering' && <CardSkeleton count={SKELETON_COUNT} />}
			{showSeam && <div className="col-span-full h-px bg-border" data-slot="results-seam" />}
			{local.map((entry) => (
				<ArrowCard entry={entry} key={entry.namespace} />
			))}
		</div>
	);
}
