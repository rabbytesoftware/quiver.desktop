import type { JSX } from 'react';

import type { DiscoverySummary } from '@/domain/search';
import type { SearchJob, SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

interface ResultsHeaderProps {
	query: string;
	count: number;
	phase: SearchPhase;
	job: SearchJob | null;
	summary: DiscoverySummary | null;
	passFailed: boolean;
	onInspect: () => void;
}

/**
 * Content, not chrome: no position and no z-index, so a hovered card's 30px
 * lift (spec 8.2) can paint over this rather than under it. `p-[18px]`
 * combines with the grid's 12px top padding (Task 7) to make that 30px.
 */
export function ResultsHeader({
	query,
	count,
	phase,
	job,
	summary,
	passFailed,
	onInspect,
}: ResultsHeaderProps): JSX.Element | null {
	const { t } = useTranslation();

	if (query === '') return null;

	const refused = summary?.providers.filter((provider) => !provider.ok) ?? [];

	return (
		<div className="p-[18px]">
			<h2 className="truncate text-[16px] font-[640] tracking-[-0.01em]">{query}</h2>
			<div
				aria-live="polite"
				className="mt-1 flex flex-wrap items-center gap-x-1.5 text-[12.5px] text-muted-foreground"
			>
				<span>{t('search.results.count', { count })}</span>
				{phase === 'discovering' && <span>· {t('search.results.searching')}</span>}
				{passFailed && <span>· {t('search.results.passFailed')}</span>}
				{refused.map((provider) => (
					<span key={provider.host}>
						· {t('search.results.refused', { host: provider.host })}
						{provider.retry_after !== null && (
							<> {t('search.results.retry', { seconds: provider.retry_after })}</>
						)}
					</span>
				))}
				{job && (
					<button className="text-foreground underline underline-offset-2" onClick={onInspect} type="button">
						{t('search.results.inspect')}
					</button>
				)}
			</div>
		</div>
	);
}
