import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeAll, describe, expect, it, vi } from 'vitest';

import { EntryRow } from './entry-row';
import type { LogRecord } from '../lib/frames';
import type { ConsoleEntry } from '../stores/console-store';

beforeAll(() => {
	vi.stubEnv('TZ', 'UTC');
});

function record(over: Partial<LogRecord> = {}): LogRecord {
	return {
		seq: 1,
		iso: '2026-10-04T14:02:14.390123Z',
		level: 'info',
		component: 'release',
		msg: 'channel lookup slow',
		fields: [
			{ key: 'ns', value: 'github.com/char2cs/crowbar', kind: 'path' },
			{ key: 'took', value: '1.8s', kind: 'duration' },
			{ key: 'retry', value: '1', kind: 'number' },
			{ key: 'auto', value: 'true', kind: 'bool' },
			{ key: 'err', value: 'boom', kind: 'error' },
			{ key: 'note', value: 'plain', kind: 'text' },
		],
		fieldsTruncated: false,
		...over,
	};
}

function renderEntry(entry: ConsoleEntry, expanded = false, onToggle = vi.fn()) {
	render(<EntryRow entry={entry} expanded={expanded} onToggle={onToggle} />);
	return onToggle;
}

describe('a daemon log line', () => {
	it('shows time, level, component and message', () => {
		renderEntry({ id: 7, kind: 'log', record: record({ level: 'warn' }) });
		expect(screen.getByText('14:02:14.390')).toBeInTheDocument();
		expect(screen.getByText('WARN')).toBeInTheDocument();
		expect(screen.getByText('release')).toBeInTheDocument();
		expect(screen.getByText('channel lookup slow')).toBeInTheDocument();
	});

	it('shows every other key as key=value, styled by the kind of value', () => {
		renderEntry({ id: 1, kind: 'log', record: record() });
		const classOf = (value: string) => screen.getByText(value).className;

		expect(classOf('github.com/char2cs/crowbar')).toContain('text-log-path');
		expect(classOf('github.com/char2cs/crowbar')).toContain('underline');
		expect(classOf('1.8s')).toContain('text-log-number');
		expect(classOf('1')).toContain('text-log-number');
		expect(classOf('true')).toContain('text-log-bool');
		expect(classOf('boom')).toContain('text-log-field-error');
		expect(classOf('plain')).toContain('text-console-foreground');
		for (const key of ['ns', 'took', 'retry', 'auto', 'err', 'note']) {
			expect(screen.getByText(key).className).toContain("after:content-['=']");
		}
	});

	it.each([
		['debug', 'DEBUG', 'text-log-debug', ''],
		['info', 'INFO', 'text-log-info', ''],
		['warn', 'WARN', 'text-log-warn', 'text-log-warn-text'],
		['error', 'ERROR', 'text-log-error', 'text-log-error-text'],
	] as const)('%s: level colour and message colour', (level, tag, levelClass, messageClass) => {
		renderEntry({ id: 1, kind: 'log', record: record({ level }) });
		expect(screen.getByText(tag).className).toContain(levelClass);
		const message = screen.getByText('channel lookup slow');
		if (messageClass) expect(message.className).toContain(messageClass);
		else expect(message.className).toBe('');
	});

	it('marks a record the daemon trimmed', () => {
		renderEntry({ id: 1, kind: 'log', record: record({ fieldsTruncated: true }) });
		expect(screen.getByText('…')).toBeInTheDocument();
	});

	it('does not mark an intact one', () => {
		renderEntry({ id: 1, kind: 'log', record: record() });
		expect(screen.queryByText('…')).toBeNull();
	});

	it('is a button that says whether it is expanded and toggles by its id', async () => {
		const onToggle = renderEntry({ id: 7, kind: 'log', record: record() });
		const row = screen.getByRole('button');
		expect(row).toHaveAttribute('aria-expanded', 'false');

		await userEvent.click(row);
		expect(onToggle).toHaveBeenCalledWith(7);
	});

	it('reveals the record as JSON when expanded, strings quoted', () => {
		renderEntry({ id: 1, kind: 'log', record: record() }, true);
		expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true');

		const json = document.querySelector('[data-slot="log-json"]') as HTMLElement;
		expect(json).not.toBeNull();
		expect(json.textContent).toContain('"time": "2026-10-04T14:02:14.390123Z"');
		expect(json.textContent).toContain('"level": "info"');
		expect(json.textContent).toContain('"msg": "channel lookup slow"');
		expect(json.textContent).toContain('"retry": 1');
		expect(json.textContent).toContain('"auto": true');
		expect(within(json).getByText('{')).toBeInTheDocument();
		expect(within(json).getByText('}')).toBeInTheDocument();
	});

	it('shows no JSON while collapsed', () => {
		renderEntry({ id: 1, kind: 'log', record: record() });
		expect(document.querySelector('[data-slot="log-json"]')).toBeNull();
	});

	it('puts a comma after every key but the last in the JSON', () => {
		renderEntry({ id: 1, kind: 'log', record: record({ fields: [] }) }, true);
		const json = document.querySelector('[data-slot="log-json"]') as HTMLElement;
		expect(json.textContent).toMatch(/"msg": "channel lookup slow"\n?\}?$/);
		expect(json.textContent).not.toContain('"msg": "channel lookup slow",');
	});
});

describe('the other lines', () => {
	it('echoes a command with the time it was typed', () => {
		renderEntry({
			id: 1,
			kind: 'cmd',
			text: 'install github.com/char2cs/crowbar',
			at: Date.UTC(2026, 9, 4, 14, 2, 9, 5),
		});
		expect(screen.getByText('14:02:09.005')).toBeInTheDocument();
		expect(screen.getByText(/install github.com\/char2cs\/crowbar/)).toBeInTheDocument();
		expect(document.querySelector('[data-slot="console-command"]')).not.toBeNull();
	});

	it('prints stdout in the normal colour and stderr in the error colour', () => {
		renderEntry({ id: 1, kind: 'out', stream: 'stdout', text: 'fetching' });
		renderEntry({ id: 2, kind: 'out', stream: 'stderr', text: 'warning: slow' });
		expect(screen.getByText('fetching').className).toContain('text-console-foreground');
		expect(screen.getByText('warning: slow').className).toContain('text-log-field-error');
		expect(screen.getByText('warning: slow')).toHaveAttribute('data-stream', 'stderr');
	});

	it('keeps the height of a blank output line', () => {
		renderEntry({ id: 1, kind: 'out', stream: 'stdout', text: '' });
		expect(document.querySelector('[data-slot="console-output"]')?.className).toContain('min-h-[18px]');
	});

	it('prints what it could not decode as plain dim text', () => {
		renderEntry({ id: 1, kind: 'raw', text: 'not json' });
		expect(screen.getByText('not json').className).toContain('text-console-dim');
	});

	it('speaks an error note as an alert, and an info note quietly', () => {
		renderEntry({ id: 1, kind: 'note', tone: 'error', note: { type: 'exit', code: 2, error: '' } });
		expect(screen.getByRole('alert')).toHaveTextContent('Exited with code 2');

		renderEntry({ id: 2, kind: 'note', tone: 'info', note: { type: 'restarted' } });
		expect(screen.getAllByRole('alert')).toHaveLength(1);
		expect(screen.getByText('Daemon restarted').className).toContain('text-console-dim');
	});

	it('colours an ok note', () => {
		renderEntry({ id: 1, kind: 'note', tone: 'ok', note: { type: 'text', text: 'done' } });
		expect(screen.getByText('done').className).toContain('text-log-bool');
	});
});
