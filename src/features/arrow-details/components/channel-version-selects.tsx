import { useState, type JSX } from 'react';

import { Input } from '@/components/ui/input';
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

	// Bumped to force the pointer-channel input below to remount -- and so
	// re-read `defaultValue` -- when a typed value must be thrown away instead
	// of kept on screen (left blank on blur/Enter). Same convention as the
	// port fields in settings/components/tabs/engine.tsx.
	const [revertNonce, setRevertNonce] = useState(0);

	function commitPointerVersion(raw: string): void {
		const value = raw.trim();
		if (!value || !selectedChannelEntry) {
			setRevertNonce((n) => n + 1);
			return;
		}
		selection.selectVersion(selectedChannelEntry.name, value);
	}

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
			{selectedChannelEntry?.kind === 'ordered' && (
				<Select
					disabled={selection.isPending}
					onValueChange={(ref) => ref && selection.selectVersion(selectedChannelEntry.name, ref)}
					value={selection.selectedVersion}
				>
					<SelectTrigger aria-label={t('arrow.version.label')} className="h-6 w-auto font-mono text-xs">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{(selectedChannelEntry.members ?? []).map((ref) => (
							<SelectItem key={ref} value={ref}>
								{ref}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			)}
			{selectedChannelEntry?.kind === 'pointer' && (
				// A pointer channel (a branch, a rolling tag) is open-ended -- there
				// is no fixed list to rank, so this pins an arbitrary ref (a commit,
				// a differently-named tag) rather than offering a closed choice of
				// one. Uncontrolled + keyed so typing isn't fought by a round trip;
				// commits on blur or Enter, never on every keystroke.
				<Input
					key={`${selectedChannelEntry.name}-${selection.selectedVersion}-${revertNonce}`}
					defaultValue={selection.selectedVersion ?? selectedChannelEntry.latest}
					disabled={selection.isPending}
					aria-label={t('arrow.version.label')}
					className="h-6 w-24 font-mono text-xs"
					onBlur={(e) => commitPointerVersion(e.target.value)}
					onKeyDown={(e) => {
						if (e.key === 'Enter') e.currentTarget.blur();
					}}
				/>
			)}
		</>
	);
}
