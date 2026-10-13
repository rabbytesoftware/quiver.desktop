import type { JSX } from 'react';

import { Button } from '@/components/ui/button';
import {
	Dialog,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogPanel,
	DialogPopup,
	DialogTitle,
} from '@/components/ui/dialog';

import type { ArrowDetail } from '@/domain/arrow';
import { useChannelSelection } from '@/features/arrow-details/lib/use-channel-selection';
import { useTranslation } from '@/lib/i18n';
import { withSelector } from '@/lib/namespace';

import { ChannelVersionSelects } from './channel-version-selects';

interface SwitchSelectorDialogProps {
	detail: ArrowDetail;
	channelsLoading: boolean;
	pending: boolean;
	/** Why registering the picked identity failed -- nothing was changed, so the dialog stays open on it. */
	error: string | null;
	onOpenChange: (open: boolean) => void;
	onConfirm: (selector: string) => void;
}

/**
 * Picks what a library entry should follow instead, and says plainly what
 * that costs: the entry's selector is fixed, so the current one is uninstalled
 * and removed and the new identity installed in its place.
 */
export function SwitchSelectorDialog({
	detail,
	channelsLoading,
	pending,
	error,
	onOpenChange,
	onConfirm,
}: SwitchSelectorDialogProps): JSX.Element {
	const { t } = useTranslation();
	const selection = useChannelSelection(detail.selector, detail.channels);
	const target = selection.selector;
	const unchanged = target === undefined || target === detail.selector;

	return (
		<Dialog onOpenChange={onOpenChange} open>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>{t('arrow.selector.switchTitle')}</DialogTitle>
					<DialogDescription>
						{channelsLoading
							? t('arrow.loading')
							: unchanged
								? t('arrow.selector.switchPick', { from: detail.namespace })
								: t('arrow.selector.switchNote', {
										from: detail.namespace,
										to: withSelector(detail.namespace, target),
									})}
					</DialogDescription>
				</DialogHeader>
				<DialogPanel>
					<div className="flex flex-wrap items-center gap-1.5 text-xs">
						<ChannelVersionSelects
							channels={detail.channels}
							channelsLoading={channelsLoading}
							disabled={pending}
							selection={selection}
						/>
					</div>
					{error && (
						<p className="mt-3 text-xs text-destructive" role="alert">
							{t('arrow.selector.registerFailed', { reason: error })}
						</p>
					)}
				</DialogPanel>
				<DialogFooter>
					<Button onClick={() => onOpenChange(false)} variant="outline">
						{t('arrow.selector.switchCancel')}
					</Button>
					<Button
						disabled={unchanged || pending || channelsLoading}
						onClick={() => target && onConfirm(target)}
						variant="destructive"
					>
						{t('arrow.selector.switchConfirm')}
					</Button>
				</DialogFooter>
			</DialogPopup>
		</Dialog>
	);
}
