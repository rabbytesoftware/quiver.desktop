import type { JSX } from 'react';

import { cn } from '@/lib/cn';
import { useTranslation, type Translator } from '@/lib/i18n';

import { LogRow } from './log-row';
import { formatLogTime } from '../lib/frames';
import type { ConsoleEntry, Note } from '../stores/console-store';

const OUTPUT =
	'px-4 pl-[114px] font-mono text-xs leading-[18px] min-h-[18px] whitespace-pre-wrap [overflow-wrap:anywhere]';

const TONE_CLASS = {
	info: 'text-console-dim',
	ok: 'text-log-bool',
	error: 'text-log-field-error',
} as const;

/** What the console says about itself, in words. */
export function noteText(note: Note, t: Translator['t']): string {
	switch (note.type) {
		case 'restarted':
			return t('console.note.restarted');
		case 'gap':
			return t('console.note.gap', { count: note.dropped });
		case 'exit':
			return note.error
				? t('console.note.exitWithError', { code: note.code, error: note.error })
				: t('console.note.exit', { code: note.code });
		case 'refused':
			return t('console.note.refused', { status: note.status, message: note.message });
		case 'unsupported':
			return t('console.note.unsupported');
		case 'helpUnavailable':
			return t('console.note.helpUnavailable');
		case 'text':
			return note.text;
	}
}

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
