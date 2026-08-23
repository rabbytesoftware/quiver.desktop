import type { JSX, ReactNode } from 'react';

import type { SearchEntry } from '@/domain/search';
import { isHeld } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

import { columnRule } from '../columns';
import { ArrowCard } from './arrow-card';
import { CardSkeleton } from './card-skeleton';

interface ResultGridProps {
	local: SearchEntry[];
	streamed: SearchEntry[];
	phase: SearchPhase;
	/**
	 * The size of the whole answer, before narrowing. The column cap is measured
	 * against this rather than against what is shown, so selecting a facet never
	 * resizes every tile on screen under the cursor.
	 */
	total: number;
}

const SKELETON_COUNT = 4;

/** pt-3 (12px) + the header's 18px (results-header.tsx) is --reveal (30px, spec 8.2/9.2). */
const GRID = 'px-[18px] pt-3 pb-[22px]';

function Shelf({ label, count, children }: { label: string; count: ReactNode; children: ReactNode }): JSX.Element {
	return (
		<section className="mt-[22px] first:mt-0">
			<div className="mb-2.5 flex h-[13px] items-center gap-2">
				<span className="font-mono text-[9.5px]/[13px] uppercase tracking-[0.09em] text-muted-foreground">
					{label}
				</span>
				<span className="font-mono text-[9.5px]/[13px] text-muted-foreground opacity-60">{count}</span>
				{/* The seam, made permanent and given a name. It used to render only
				    while the pass ran and dissolved at the settle -- exactly when the
				    list became the one you decide from. Spec 9.3. */}
				<span className="h-px flex-1 bg-border" />
			</div>
			{children}
		</section>
	);
}

export function ResultGrid({ local, streamed, phase, total }: ResultGridProps): JSX.Element {
	const { t } = useTranslation();

	// Lane A is ranked and Lane B is arrival order, so concatenating in that
	// order keeps the ranked rows at the top of whichever shelf they land in.
	const all = [...local, ...streamed];

	// Keyed on the entry, never on the lane: a discovered arrow the catalog
	// already holds streams with `installed: true`, and Lane A may have ranked
	// it below the limit, so the streamed band is not "things you lack".
	const held = all.filter(isHeld);
	const rest = all.filter((entry) => !isHeld(entry));

	const columns = { gridTemplateColumns: columnRule(total) };
	const discovering = phase === 'discovering';

	return (
		<div className={GRID}>
			{held.length > 0 && (
				<Shelf count={held.length} label={t('search.shelf.vault')}>
					<div className="grid gap-x-3 gap-y-[18px]" style={columns}>
						{held.map((entry) => (
							<ArrowCard entry={entry} key={entry.namespace} />
						))}
					</div>
				</Shelf>
			)}
			{(rest.length > 0 || discovering) && (
				<Shelf
					count={discovering ? t('search.shelf.soFar', { count: rest.length }) : rest.length}
					label={t('search.shelf.network')}
				>
					<div className="grid gap-x-3 gap-y-[18px]" style={columns}>
						{rest.map((entry) => (
							<ArrowCard entry={entry} key={entry.namespace} />
						))}
						{discovering && <CardSkeleton count={SKELETON_COUNT} />}
					</div>
				</Shelf>
			)}
		</div>
	);
}
