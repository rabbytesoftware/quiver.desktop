import type { JSX } from 'react';

import type { ConsoleEntry } from '@/features/console/lib/entries';
import { formatLogTime } from '@/features/console/lib/frames';
import { noteText } from '@/features/console/lib/notes';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import { LogRow } from './log-row';

const OUTPUT =
	'px-4 pl-[114px] font-mono text-xs leading-[18px] min-h-[18px] whitespace-pre-wrap [overflow-wrap:anywhere]';

const TONE_CLASS = {
	info: 'text-console-dim',
	ok: 'text-log-bool',
	error: 'text-log-field-error',
} as const;

export interface EntryRowProps {
	entry: ConsoleEntry;
	expanded: boolean;
	onToggle: (id: number) => void;
}

/** Any line of the console: a daemon log, a command typed, what it printed, or a note from the console itself. */
export function EntryRow({ entry, expanded, onToggle }: EntryRowProps): JSX.Element {
	const { t } = useTranslation();

	switch (entry.kind) {
		case 'log':
			return <LogRow id={entry.id} record={entry.record} expanded={expanded} onToggle={onToggle} />;
		case 'cmd':
			return (
				<div
					data-slot="console-command"
					className="grid grid-cols-[88px_minmax(0,1fr)] gap-x-2.5 px-4 pt-2 pb-0.5 font-mono text-[12.5px] leading-[18px]"
				>
					<span className="text-console-dim">{formatLogTime(new Date(entry.at).toISOString())}</span>
					<span className="font-medium [overflow-wrap:anywhere]">
						<span aria-hidden="true">› </span>
						{entry.text}
					</span>
				</div>
			);
		case 'out':
			return (
				<div
					data-slot="console-output"
					data-stream={entry.stream}
					className={cn(
						OUTPUT,
						entry.stream === 'stderr' ? 'text-log-field-error' : 'text-console-foreground'
					)}
				>
					{entry.text}
				</div>
			);
		case 'raw':
			return (
				<div data-slot="console-raw" className={cn(OUTPUT, 'text-console-dim')}>
					{entry.text}
				</div>
			);
		case 'note':
			return (
				<div
					data-slot="console-note"
					data-tone={entry.tone}
					role={entry.tone === 'error' ? 'alert' : undefined}
					className={cn(OUTPUT, TONE_CLASS[entry.tone])}
				>
					{noteText(entry.note, t)}
				</div>
			);
	}
}
