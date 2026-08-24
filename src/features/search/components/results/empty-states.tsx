import type { JSX } from 'react';

import type { DiscoverySummary } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

interface EmptyStateProps {
	/** The committed query, blank when nothing has been asked for. */
	query: string;
	phase: SearchPhase;
	summary: DiscoverySummary | null;
	localError: boolean;
	hasResults: boolean;
	passFailed: boolean;
}

// The design's one box for all of these: centred in its own height rather than
// pushed down from the heading, which the resting screen does not have.
const WRAP = [
	'flex min-h-[220px] flex-col items-center justify-center gap-1',
	'px-10 text-center text-[12.5px] text-muted-foreground',
].join(' ');

export function EmptyState({
	query,
	phase,
	summary,
	localError,
	hasResults,
	passFailed,
}: EmptyStateProps): JSX.Element | null {
	const { t } = useTranslation();

	if (hasResults) return null;
	if (localError) return <p className={WRAP}>{t('search.results.unreachable')}</p>;

	// Nothing has been asked for. There is no heading and no grid on this screen,
	// so without a line here it is a blank canvas. Gated on the query rather than
	// on `idle`, because `setQuery` empties the store before Lane A answers and a
	// real query passes through `idle` on its way in.
	if (query.trim() === '') return <p className={WRAP}>{t('search.results.idle')}</p>;
	// The local window: Lane A has answered but no host has been asked yet --
	// "every host answered" would claim a network pass that hasn't started.
	if (phase === 'idle' || phase === 'local' || phase === 'discovering') return null;

	// A timeout is a claim the network never got to make (spec 10.2); the
	// header already states the failure, so this follows the refusal
	// branch's rule below and says nothing.
	if (passFailed) return null;

	const refused = summary?.providers.filter((provider) => !provider.ok) ?? [];

	// The refusal itself is in the header trigger and the sheet (spec 9.3), but
	// that used to be a clause in this line and the screen went blank without it.
	// Says what happened without claiming every host answered.
	if (refused.length > 0) return <p className={WRAP}>{t('search.results.emptyWithRefusal')}</p>;

	return <p className={WRAP}>{t('search.results.emptyEverywhere')}</p>;
}
