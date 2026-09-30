import type { JSX } from 'react';

import { Button } from '@/components/ui/button';

import type { ArrowDetail } from '@/domain/arrow';
import { canSwitch } from '@/features/arrow-details/lib/switch-selector';
import { useTranslation } from '@/lib/i18n';

interface SelectorSummaryProps {
	detail: ArrowDetail;
	/** Opens the switch dialog. Omitted when there is nothing published to switch to. */
	onSwitch?: () => void;
	pending: boolean;
}

/**
 * What a library entry follows, read-only: its selector (fixed for the life
 * of the entry) and the ref that resolves to. Changing it is the explicit
 * switch flow, never an edit in place.
 */
export function SelectorSummary({ detail, onSwitch, pending }: SelectorSummaryProps): JSX.Element {
	const { t } = useTranslation();
	const resolved = detail.resolved_ref && detail.resolved_ref !== detail.selector ? detail.resolved_ref : null;

	return (
		<span className="flex items-center gap-1.5" data-slot="selector-summary">
			<span>{t(`arrow.selector.kind.${detail.selector_kind}`)}</span>
			<span className="font-mono text-foreground">{detail.selector}</span>
			{resolved && <span className="font-mono">({t('arrow.selector.resolved', { ref: resolved })})</span>}
			{onSwitch && (
				<Button
					className="h-6"
					disabled={pending || !canSwitch(detail.state)}
					onClick={onSwitch}
					size="xs"
					variant="ghost"
				>
					{t('arrow.selector.switch')}
				</Button>
			)}
		</span>
	);
}
