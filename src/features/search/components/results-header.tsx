import type { JSX, ReactNode } from 'react';

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

import type { DiscoverySummary } from '@/domain/search';
import type { SearchJob, SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

import type { SortKey } from '../sort';
import { SORT_KEYS } from '../sort';

interface ResultsHeaderProps {
	query: string;
	count: number;
	phase: SearchPhase;
	job: SearchJob | null;
	summary: DiscoverySummary | null;
	passFailed: boolean;
	sort: SortKey;
	onSortChange: (sort: SortKey) => void;
	onInspect: () => void;
	/** The narrow row (spec 9.8). Passed in so the header does not have to know
	 *  what narrowing is -- it only owns the space. */
	facets?: ReactNode;
}

const TRIGGER = [
	'inline-flex h-[23px] cursor-pointer items-center gap-1.5 rounded-[7px] border px-2',
	'text-[11.5px]/[15px] text-muted-foreground',
	'transition-colors hover:bg-sidebar-element-idle',
	'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
].join(' ');

/**
 * Content, not chrome: no position and no z-index, so a hovered card's 30px
 * lift (spec 8.2) can paint over this rather than under it. `p-[18px]`
 * combines with the grid's 12px top padding (Task 7) to make that 30px.
 *
 * The query is not repeated here. The sidebar field holds it, inverted, eighteen
 * pixels away -- spec 9.2.
 */
export function ResultsHeader({
	query,
	count,
	phase,
	job,
	summary,
	passFailed,
	sort,
	onSortChange,
	onInspect,
	facets,
}: ResultsHeaderProps): JSX.Element | null {
	const { t } = useTranslation();

	if (query === '') return null;

	const refused = summary?.providers.filter((provider) => !provider.ok) ?? [];
	const running = phase === 'discovering';
	// Reordering a list that is still arriving would move cards out from under
	// the cursor, and there is nothing to reorder before Lane A answers.
	const sortable = phase === 'settled' || phase === 'local';

	const options = SORT_KEYS.map((key) => ({ value: key, label: t(`search.sort.${key}`) }));

	return (
		<div className="p-[18px]">
			<div className="flex items-center gap-3">
				<span aria-live="polite" className="flex-none text-[12.5px]/[16px] text-muted-foreground">
					{t('search.results.count', { count })}
					{running && ` · ${t('search.results.searching')}`}
					{passFailed && ` · ${t('search.results.passFailed')}`}
				</span>

				{facets}

				<div className="ml-auto flex flex-none items-center gap-3.5">
					{/* Everything the pass did lives one click away (spec 11), and a
					    refusal stays legible at rest -- it used to be a wrapping grey
					    clause that a hovered card painted straight over. Mid-pass it
					    still opens: the sheet says why it is empty (spec 11.4). */}
					{(job !== null || summary !== null) && (
						<button className={TRIGGER} onClick={onInspect} type="button">
							<svg
								aria-hidden="true"
								className="block opacity-70"
								fill="none"
								height="11"
								stroke="currentColor"
								strokeWidth="1.3"
								viewBox="0 0 14 14"
								width="11"
							>
								<rect height="9.2" rx="1.8" width="11.2" x="1.4" y="2.4" />
								<path d="M9.1 2.4v9.2" />
							</svg>
							{summary === null ? (
								<span className="font-medium text-foreground">{t('search.results.inspect')}</span>
							) : (
								<>
									<span className="font-medium text-foreground">
										{t('search.results.hosts', { count: summary.providers.length })}
									</span>
									{refused.length > 0 && (
										<span>· {t('search.results.refusedCount', { count: refused.length })}</span>
									)}
								</>
							)}
						</button>
					)}

					{sortable && count > 1 && (
						<Select items={options} onValueChange={(next) => onSortChange(next as SortKey)} value={sort}>
							<SelectTrigger
								aria-label={t('search.sort.label')}
								className="h-[23px] w-[126px] text-[11.5px]"
							>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{options.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										{option.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					)}
				</div>
			</div>
		</div>
	);
}
