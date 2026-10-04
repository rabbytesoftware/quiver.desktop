import { memo, type JSX } from 'react';

import { formatLogTime, jsonLines, type FieldKind, type LogLevel, type LogRecord } from '@/features/console/lib/frames';
import { cn } from '@/lib/cn';

const LEVEL_CLASS: Record<LogLevel, string> = {
	debug: 'text-log-debug',
	info: 'text-log-info',
	warn: 'text-log-warn',
	error: 'text-log-error',
};

// Only warnings and errors colour their message: a problem is the one line
// that is not grey.
const MESSAGE_CLASS: Record<LogLevel, string> = {
	debug: '',
	info: '',
	warn: 'text-log-warn-text',
	error: 'text-log-error-text',
};

const FIELD_CLASS: Record<FieldKind, string> = {
	text: 'text-console-foreground',
	number: 'text-log-number',
	duration: 'text-log-number',
	bool: 'text-log-bool',
	path: 'text-log-path underline decoration-log-underline underline-offset-2',
	error: 'text-log-field-error',
};

export interface LogRowProps {
	id: number;
	record: LogRecord;
	expanded: boolean;
	onToggle: (id: number) => void;
}

/**
 * One daemon log line: time, level, then the message and every other key as
 * `key=value`. The component is not a column of its own (most lines have none,
 * which left a gap after the level); it is in the raw JSON, one click away.
 */
export const LogRow = memo(function LogRow({ id, record, expanded, onToggle }: LogRowProps): JSX.Element {
	return (
		<div data-slot="log-row" data-level={record.level}>
			<button
				type="button"
				aria-expanded={expanded}
				onClick={() => onToggle(id)}
				className={cn(
					'grid w-full cursor-pointer grid-cols-[88px_44px_minmax(0,1fr)] gap-x-2.5 px-4 py-[3px] text-left font-mono text-xs leading-[18px] text-console-foreground outline-none hover:bg-console-row-hover focus-visible:bg-console-row-hover',
					expanded && 'bg-console-row-open'
				)}
			>
				<span className="text-console-dim">{formatLogTime(record.iso)}</span>
				<span className={cn('font-semibold tracking-[0.04em]', LEVEL_CLASS[record.level])}>
					{record.level.toUpperCase()}
				</span>
				<span className="[overflow-wrap:anywhere]">
					<span className={MESSAGE_CLASS[record.level]}>{record.msg}</span>
					{record.fields.map((field) => (
						<span key={field.key} className="ml-2.5 inline-block">
							<span className="text-console-dim after:content-['=']">{field.key}</span>
							<span className={FIELD_CLASS[field.kind]}>{field.value}</span>
						</span>
					))}
					{record.fieldsTruncated && <span className="ml-2.5 text-console-dim">…</span>}
				</span>
			</button>
			{expanded && <JsonView record={record} />}
		</div>
	);
});

function JsonView({ record }: { record: LogRecord }): JSX.Element {
	return (
		<div
			data-slot="log-json"
			className="bg-console-row-open py-1.5 pr-4 pl-[168px] font-mono text-xs leading-[18px] text-console-dim"
		>
			<div>{'{'}</div>
			{jsonLines(record).map((line, i, all) => (
				<div key={line.key} className="pl-4">
					<span>&quot;{line.key}&quot;</span>
					<span>: </span>
					<span className={FIELD_CLASS[line.kind]}>{line.value}</span>
					{i < all.length - 1 && <span>,</span>}
				</div>
			))}
			<div>{'}'}</div>
		</div>
	);
}
