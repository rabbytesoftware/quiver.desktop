import type { JSX } from 'react';

import type { DiscoverySummary } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

interface EmptyStateProps {
	phase: SearchPhase;
	summary: DiscoverySummary | null;
	localError: boolean;
}

const WRAP = 'flex flex-col items-center gap-1 pt-24 text-center text-sm text-muted-foreground';

export function EmptyState({ phase, summary, localError }: EmptyStateProps): JSX.Element | null {
	const { t } = useTranslation();

	if (localError) return <p className={WRAP}>{t('search.results.unreachable')}</p>;
	// The local window: Lane A has answered but no host has been asked yet --
	// "every host answered" would claim a network pass that hasn't started.
	if (phase === 'idle' || phase === 'local' || phase === 'discovering') return null;

	const refused = summary?.providers.filter((provider) => !provider.ok) ?? [];

	// A refusal means the search never happened at that host -- never fall
	// through to "nothing matched" for it.
	if (refused.length > 0) {
		return (
			<div className={WRAP}>
				{refused.map((provider) => (
					<p key={provider.host}>
						{t('search.results.refused', { host: provider.host })}
						{provider.reason !== null && `: ${provider.reason}`}
						{provider.retry_after !== null &&
							` · ${t('search.results.retry', { seconds: provider.retry_after })}`}
					</p>
				))}
			</div>
		);
	}

	return <p className={WRAP}>{t('search.results.emptyEverywhere')}</p>;
}
