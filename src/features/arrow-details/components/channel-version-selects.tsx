import type { JSX } from 'react';

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

import type { ArrowChannel } from '@/domain/arrow';
import type { ChannelSelection } from '@/features/arrow-details/lib/use-channel-selection';
import { useTranslation } from '@/lib/i18n';

interface ChannelVersionSelectsProps {
	channels: ArrowChannel[];
	selection: ChannelSelection;
}

/**
 * The Hero's Channel and Version pickers -- pulled out of `Hero` itself
 * (react-doctor's `no-giant-component`) since the two selects, their pointer-
 * channel handling, and the version-option derivation are one self-contained
 * concern. `channels` is the arrow's full published list; the rest of the
 * current selection lives in `selection`, already resolved by `useChannelSelection`.
 */
export function ChannelVersionSelects({ channels, selection }: ChannelVersionSelectsProps): JSX.Element {
	const { t } = useTranslation();
	const { selectedChannelEntry } = selection;

	return (
		<>
			{channels.length > 0 && (
				<Select
					disabled={selection.isPending}
					onValueChange={(name) => name && selection.selectChannel(name)}
					value={selection.selectedChannel}
				>
					<SelectTrigger aria-label={t('arrow.channel.label')} className="h-6 w-auto font-mono text-xs">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{channels.map((c) => (
							<SelectItem key={c.name} value={c.name}>
								{c.kind === 'pointer' ? `${c.name} (${t('arrow.channel.pointer')})` : c.name}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			)}
			{selectedChannelEntry && (
				// A pointer channel has nothing to rank -- its own `latest` is the
				// only version, so the select stays visible (for the same visual
				// rhythm as the ordered case) but disabled rather than offering a
				// choice of one.
				<Select
					disabled={selection.isPending || selectedChannelEntry.kind === 'pointer'}
					onValueChange={(ref) => ref && selection.selectVersion(selectedChannelEntry.name, ref)}
					value={selection.selectedVersion}
				>
					<SelectTrigger aria-label={t('arrow.version.label')} className="h-6 w-auto font-mono text-xs">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{(selectedChannelEntry.kind === 'ordered'
							? (selectedChannelEntry.members ?? [])
							: [selectedChannelEntry.latest]
						).map((ref) => (
							<SelectItem key={ref} value={ref}>
								{ref}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			)}
		</>
	);
}
