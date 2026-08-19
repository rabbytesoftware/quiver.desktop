import type { JSX } from 'react';

import type { DiscoverySummary } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

interface EmptyStateProps {
	phase: SearchPhase;
	summary: DiscoverySummary | null;
	localError: boolean;
	hasResults: boolean;
}

const WRAP = 'flex flex-col items-center gap-1 pt-24 text-center text-sm text-muted-foreground';

export function EmptyState({ phase, summary, localError, hasResults }: EmptyStateProps): JSX.Element | null {
	const { t } = useTranslation();

	if (hasResults) return null;
	if (localError) return <p className={WRAP}>{t('search.results.unreachable')}</p>;
	// The local window: Lane A has answered but no host has been asked yet --
	// "every host answered" would claim a network pass that hasn't started.
	if (phase === 'idle' || phase === 'local' || phase === 'discovering') return null;

	const refused = summary?.providers.filter((provider) => !provider.ok) ?? [];

	// The meta line already states the refusal and its retry_after (spec 10.2);
	// this component's only job left is to not claim "every host answered".
	if (refused.length > 0) return null;

	return <p className={WRAP}>{t('search.results.emptyEverywhere')}</p>;
}
