import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ConsoleCommand } from '@/features/console/lib/commands';
import { useConsoleStore } from '@/features/console/stores/console-store';

import { ConsolePrompt } from './console-prompt';

const COMMANDS: ConsoleCommand[] = [
	{ path: ['install'], short: '', usage: 'install', aliases: [], flags: [] },
	{ path: ['info'], short: '', usage: 'info', aliases: [], flags: [] },
	{ path: ['list'], short: '', usage: 'list', aliases: [], flags: [] },
];

const input = () => screen.getByRole('textbox') as HTMLInputElement;
const store = () => useConsoleStore.getState();

beforeEach(() => {
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
	store().setCommands(COMMANDS);
});

function renderPrompt(over: { onSubmit?: (line: string) => void; focused?: boolean } = {}) {
	const onSubmit = over.onSubmit ?? vi.fn();
	render(<ConsolePrompt onSubmit={onSubmit} focused={over.focused ?? false} />);
	return onSubmit;
}

describe('the command line', () => {
	it('is labelled and offers a hint', () => {
		renderPrompt();
		expect(screen.getByRole('textbox', { name: 'Command' })).toBe(input());
		// The hint is a command the daemon accepts: there is no bare `add`.
		expect(input()).toHaveAttribute('placeholder', 'info github.com/rabbytesoftware/quiver.core');
	});

	it('does not let the OS fix what is typed', () => {
		renderPrompt();
		expect(input()).toHaveAttribute('autocomplete', 'off');
		expect(input()).toHaveAttribute('autocapitalize', 'off');
		expect(input()).toHaveAttribute('autocorrect', 'off');
		expect(input()).toHaveAttribute('spellcheck', 'false');
	});

	it('keeps what is typed in the store', async () => {
		renderPrompt();
		await userEvent.type(input(), 'list');
		expect(store().draft).toBe('list');
		expect(input()).toHaveValue('list');
	});

	it('submits the line on Enter', async () => {
		const onSubmit = renderPrompt();
		await userEvent.type(input(), 'install x{Enter}');
		expect(onSubmit).toHaveBeenCalledWith('install x');
	});

	it('takes focus when the console is shown, and not before', () => {
		const { rerender } = render(<ConsolePrompt onSubmit={() => {}} focused={false} />);
		expect(input()).not.toHaveFocus();
		rerender(<ConsolePrompt onSubmit={() => {}} focused={true} />);
		expect(input()).toHaveFocus();
	});

	it('shows a reconnect notice only while the stream is reconnecting', () => {
		renderPrompt();
		expect(screen.queryByRole('status')).toBeNull();
		act(() => store().setStream('reconnecting'));
		expect(screen.getByRole('status')).toHaveTextContent('Reconnecting…');
	});
});

describe('history', () => {
	beforeEach(() => {
		store().remember('one');
		store().remember('two');
	});

	it('Up walks back, Down walks forward and restores the draft', async () => {
		renderPrompt();
		await userEvent.type(input(), 'half');
		await userEvent.keyboard('{ArrowUp}');
		expect(input()).toHaveValue('two');
		await userEvent.keyboard('{ArrowUp}');
		expect(input()).toHaveValue('one');
		await userEvent.keyboard('{ArrowDown}{ArrowDown}');
		expect(input()).toHaveValue('half');
	});

	it('claims the arrow keys, so the caret does not jump', () => {
		renderPrompt();
		expect(fireEvent.keyDown(input(), { key: 'ArrowUp' })).toBe(false);
		expect(fireEvent.keyDown(input(), { key: 'ArrowDown' })).toBe(false);
	});

	it('leaves keys alone while an input method is composing', () => {
		renderPrompt();
		fireEvent.keyDown(input(), { key: 'ArrowUp', isComposing: true });
		expect(input()).toHaveValue('');
	});
});

describe('Tab completion', () => {
	it("completes a unique command word from the daemon's table", async () => {
		renderPrompt();
		await userEvent.type(input(), 'li');
		await userEvent.keyboard('{Tab}');
		expect(input()).toHaveValue('list ');
	});

	it('completes as far as the matches agree', async () => {
		renderPrompt();
		await userEvent.type(input(), 'i');
		await userEvent.keyboard('{Tab}');
		expect(input()).toHaveValue('in');
	});

	it('lists the matches when it cannot go further', async () => {
		renderPrompt();
		await userEvent.type(input(), 'in');
		await userEvent.keyboard('{Tab}');
		expect(input()).toHaveValue('in');
		expect(store().entries[store().entries.length - 1]).toMatchObject({ kind: 'out', text: 'info  install' });
	});

	it('does not swallow Tab when there is nothing to complete', () => {
		renderPrompt();
		fireEvent.change(input(), { target: { value: 'zzz' } });
		expect(fireEvent.keyDown(input(), { key: 'Tab' })).toBe(true);
		expect(input()).toHaveValue('zzz');
	});

	it('does not complete without a command table', () => {
		useConsoleStore.getState().setCommands([]);
		renderPrompt();
		fireEvent.change(input(), { target: { value: 'li' } });
		expect(fireEvent.keyDown(input(), { key: 'Tab' })).toBe(true);
	});
});

describe('Escape', () => {
	it('is left to the panel around the prompt, which closes the console', () => {
		store().setOpen(true);
		renderPrompt();
		// The prompt does not handle it itself: the key is neither claimed nor acted on here.
		expect(fireEvent.keyDown(input(), { key: 'Escape' })).toBe(true);
		expect(store().open).toBe(true);
	});
});

describe('other keys', () => {
	it('are left to the input', () => {
		renderPrompt();
		expect(fireEvent.keyDown(input(), { key: 'a' })).toBe(true);
	});
});
