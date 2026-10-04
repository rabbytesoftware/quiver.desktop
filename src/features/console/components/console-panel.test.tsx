import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useConsoleStore } from '../stores/console-store';
import { stubLayout } from '../test-layout';
import { ConsolePanel } from './console-panel';

const submit = vi.fn();
vi.mock('../lib/instance', () => ({ getConsoleController: () => ({ submit }) }));

let restoreLayout: () => void;

beforeEach(() => {
	restoreLayout = stubLayout();
	submit.mockReset();
	useConsoleStore.setState(useConsoleStore.getInitialState(), true);
});

afterEach(() => restoreLayout());

const panel = () => screen.getByRole('region', { name: 'Daemon console', hidden: true });

describe('the console panel', () => {
	it('is the target of the build indicator', () => {
		render(<ConsolePanel />);
		expect(panel()).toHaveAttribute('id', 'console-panel');
	});

	it('is down when open and unreachable when closed', () => {
		useConsoleStore.setState({ open: true });
		const { rerender } = render(<ConsolePanel />);
		expect(panel()).toHaveAttribute('data-open', 'true');
		expect(panel()).not.toHaveAttribute('inert');
		expect(panel().className).toContain('translate-y-0');

		useConsoleStore.setState({ open: false });
		rerender(<ConsolePanel />);
		expect(panel()).toHaveAttribute('data-open', 'false');
		expect(panel()).toHaveAttribute('inert');
		expect(panel().className).toContain('invisible');
		expect(panel().className).toContain('-translate-y-full');
	});

	it('shows the log and the prompt, and nothing else', () => {
		useConsoleStore.setState({ open: true });
		render(<ConsolePanel />);
		expect(screen.getByRole('log')).toBeInTheDocument();
		expect(screen.getByRole('textbox')).toBeInTheDocument();
		expect(within(panel()).queryAllByRole('button')).toHaveLength(0);
	});

	it('shows what the store holds', () => {
		useConsoleStore.setState({ open: true });
		useConsoleStore.getState().push([
			{ kind: 'cmd', text: 'list', at: 0 },
			{ kind: 'out', stream: 'stdout', text: 'github.com/char2cs/crowbar' },
		]);
		render(<ConsolePanel />);
		expect(screen.getByText('github.com/char2cs/crowbar')).toBeInTheDocument();
	});

	it('expands a log line on click', async () => {
		useConsoleStore.setState({ open: true });
		useConsoleStore.getState().ingest([
			{
				type: 'log',
				record: {
					seq: 1,
					iso: '2026-10-04T14:02:14Z',
					level: 'info',
					component: 'daemon',
					msg: 'daemon listening',
					fields: [],
					fieldsTruncated: false,
				},
			},
		]);
		render(<ConsolePanel />);

		await userEvent.click(screen.getByRole('button', { name: /daemon listening/ }));
		expect(useConsoleStore.getState().expandedId).toBe(1);
	});

	it('runs what is typed through the controller', async () => {
		useConsoleStore.setState({ open: true });
		render(<ConsolePanel />);
		await userEvent.type(screen.getByRole('textbox'), 'list{Enter}');
		expect(submit).toHaveBeenCalledWith('list');
	});

	it('says plainly that a daemon has no console, and offers no controls', () => {
		useConsoleStore.setState({ open: true, support: 'unsupported' });
		render(<ConsolePanel />);
		expect(screen.getByText('This daemon has no console. Update quiver.core to use it.')).toBeInTheDocument();
		expect(screen.queryByRole('textbox')).toBeNull();
		expect(screen.queryByRole('log')).toBeNull();
	});

	it('still offers the console while it does not yet know', () => {
		useConsoleStore.setState({ open: true, support: 'unknown' });
		render(<ConsolePanel />);
		expect(screen.getByRole('textbox')).toBeInTheDocument();
	});
});
