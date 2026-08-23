import type { JSX } from 'react';

import { Badge } from '@/components/ui/badge';

import type { SearchEntry } from '@/domain/search';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import type { FacetKind, Selection } from '../narrow';
import { facetsFor, isNarrowed } from '../narrow';

interface SearchFacetsProps {
	/**
	 * The whole answer, not the narrowed one: a facet that disappears when you
	 * select it cannot be unselected.
	 */
	entries: SearchEntry[];
	selection: Selection;
	onToggle: (kind: FacetKind, value: string) => void;
	onClear: () => void;
}

/** Hosts are few and coarse; tags are the long tail, so they get the room. */
const TAKE: Record<FacetKind, number> = { host: 4, tag: 12 };

/** Below this the answer is readable as it stands and the row is just chrome. */
const MIN_TO_NARROW = 4;

const SCROLLER = [
	'flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto overscroll-x-contain py-0.5',
	// The row is the only thing between the count and the pass trigger, so it
	// scrolls rather than wrapping the header to a second line.
	'[scrollbar-width:none] [&::-webkit-scrollbar]:hidden',
].join(' ');

/**
 * Spec 9.8. Set-reducing controls were always the half of spec 17 that "would
 * do real work"; what blocked them was a card that said nothing at rest.
 *
 * One row rather than a labelled rail: hosts and tags are both just "narrow by
 * this", the values say which they are, and a 240px column holding eight short
 * rows cost the grid a whole column of art.
 */
export function SearchFacets({ entries, selection, onToggle, onClear }: SearchFacetsProps): JSX.Element | null {
	const { t } = useTranslation();

	const items = [
		...facetsFor(entries, 'host', TAKE.host).map((facet) => ({ kind: 'host' as const, ...facet })),
		...facetsFor(entries, 'tag', TAKE.tag).map((facet) => ({ kind: 'tag' as const, ...facet })),
	];

	if (entries.length < MIN_TO_NARROW || items.length === 0) return null;

	const chosen = { host: new Set(selection.host), tag: new Set(selection.tag) };

	return (
		<>
			<div
				aria-label={t('search.narrow.label')}
				className={SCROLLER}
				role="group"
				// Fades the trailing edge so a row that runs past the trigger reads as
				// scrollable without a scrollbar. Invisible when it does not overflow,
				// because the leftover space is empty.
				style={{
					maskImage: 'linear-gradient(to right, black calc(100% - 22px), transparent)',
					WebkitMaskImage: 'linear-gradient(to right, black calc(100% - 22px), transparent)',
				}}
			>
				{items.map((item) => {
					const on = chosen[item.kind].has(item.value);
					return (
						<Badge
							key={`${item.kind}:${item.value}`}
							render={
								<button
									aria-label={t('search.narrow.facet', { count: item.count, value: item.value })}
									aria-pressed={on}
									onClick={() => onToggle(item.kind, item.value)}
									type="button"
								/>
							}
							size="lg"
							variant={on ? 'default' : 'outline'}
						>
							{item.value}
							<span className={cn('tabular-nums', on ? 'opacity-64' : 'text-muted-foreground')}>
								{item.count}
							</span>
						</Badge>
					);
				})}
			</div>

			{/* Outside the scroller, so it never scrolls out of reach. */}
			{isNarrowed(selection) && (
				<button
					className="flex-none cursor-pointer text-[11.5px] text-muted-foreground underline underline-offset-2 hover:text-foreground"
					onClick={onClear}
					type="button"
				>
					{t('search.narrow.clear')}
				</button>
			)}
		</>
	);
}
