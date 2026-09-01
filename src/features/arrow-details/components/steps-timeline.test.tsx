import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import type { ActiveRun, LastReturn, StepProgress } from '@/domain/arrow';

import { StepsTimeline } from './steps-timeline';

function step(overrides: Partial<StepProgress>): StepProgress {
	return { index: 0, status: 'pending', title: 'Step', type: 'run', ...overrides };
}

describe('StepsTimeline', () => {
	describe('header', () => {
		it.each([
			['install', 'Install'],
			['uninstall', 'Uninstall'],
			['update', 'Update'],
			['execute', 'Start'],
			['stop', 'Stop'],
			['backup', 'backup'],
		])('resolves an active run method %s to the label %s', (method, label) => {
			const activeRun: ActiveRun = { method, steps: [], variables: {} };
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			expect(screen.getByText(label)).toBeInTheDocument();
		});

		it('shows no description while a run is live', () => {
			const activeRun: ActiveRun = { method: 'install', steps: [], variables: {} };
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			expect(document.querySelector('[data-slot="frame-panel-description"]')).not.toBeInTheDocument();
		});

		it('falls back to the generic Activity title when there is neither a run nor a last outcome', () => {
			render(<StepsTimeline activeRun={null} lastReturn={null} userInstalled />);
			expect(screen.getByText('Activity')).toBeInTheDocument();
		});

		it.each([
			['success', 'Succeeded'],
			['failed', 'Failed'],
			['cancelled', 'Cancelled'],
		] as const)('shows the %s outcome as both a header description and a summary row', (outcome, label) => {
			const lastReturn: LastReturn = { method: 'update', outcome };
			render(<StepsTimeline activeRun={null} lastReturn={lastReturn} userInstalled />);

			expect(screen.getByText('Update')).toBeInTheDocument();
			const description = document.querySelector('[data-slot="frame-panel-description"]');
			expect(description).toHaveTextContent(label);
			expect(screen.getAllByText(label)).toHaveLength(2);
		});
	});

	describe('empty state', () => {
		it('shows the not-installed message for a Discovered arrow', () => {
			render(<StepsTimeline activeRun={null} lastReturn={null} userInstalled={false} />);
			expect(screen.getByText('Add this arrow to your library to install and run it.')).toBeInTheDocument();
		});

		it('shows the generic empty message for an installed arrow with no history', () => {
			render(<StepsTimeline activeRun={null} lastReturn={null} userInstalled />);
			expect(screen.getByText('No activity yet.')).toBeInTheDocument();
		});
	});

	describe('live steps', () => {
		const STEPS: StepProgress[] = [
			step({ index: 0, status: 'completed', title: 'Fetch jar', type: 'fetch' }),
			step({ index: 1, status: 'running', title: 'Run installer', type: 'run' }),
			step({ index: 2, status: 'pending', title: 'Signal ready', type: 'signal' }),
			step({ index: 3, status: 'pending', title: 'Unpack', type: 'exec' }),
			step({ error: 'checksum mismatch', index: 4, status: 'failed', title: 'Verify checksum', type: 'run' }),
		];
		const activeRun: ActiveRun = { method: 'install', steps: STEPS, variables: {} };

		it('renders every step with its title and its raw (untranslated) type label', () => {
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);

			for (const s of STEPS) expect(screen.getByText(s.title)).toBeInTheDocument();

			expect(screen.getByText('fetch')).toBeInTheDocument();
			expect(screen.getByText('signal')).toBeInTheDocument();
			// Real fixture data uses types outside the four known kinds -- shown
			// as-is rather than crashing or being hidden.
			expect(screen.getByText('exec')).toBeInTheDocument();
			// 'run' appears twice: "Run installer" and "Verify checksum".
			expect(screen.getAllByText('run')).toHaveLength(2);
		});

		it('shows the 1-based index inside a pending step’s dot', () => {
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			// index 2 ("Signal ready") displays as 3, index 3 ("Unpack") as 4.
			expect(screen.getByText('3')).toBeInTheDocument();
			expect(screen.getByText('4')).toBeInTheDocument();
		});

		it('shows the failed step’s error message and no error block for the others', () => {
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			expect(screen.getByText('checksum mismatch')).toBeInTheDocument();
			expect(document.querySelectorAll('pre')).toHaveLength(1);
		});

		it('draws a connecting rail between steps but not after the last one', () => {
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			expect(document.querySelectorAll('.w-px')).toHaveLength(STEPS.length - 1);
		});

		it('opens the inspect modal for the clicked step’s raw type/title, and closes it again', async () => {
			const user = userEvent.setup();
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);

			const inspectButtons = screen.getAllByRole('button', { name: 'Inspect step definition' });
			await user.click(inspectButtons[1]);

			expect(await screen.findByText('Raw step definition')).toBeInTheDocument();
			const pre = document.querySelector('[data-slot="dialog-panel"] pre');
			expect(pre).toHaveTextContent('type: run');
			expect(pre).toHaveTextContent('title: Run installer');

			await user.click(screen.getByRole('button', { name: 'Close' }));
			await waitFor(() => expect(screen.queryByText('Raw step definition')).not.toBeInTheDocument());
		});

		it('renders nothing in the empty-state paragraph while a run is live', () => {
			render(<StepsTimeline activeRun={activeRun} lastReturn={null} userInstalled />);
			expect(screen.queryByText('No activity yet.')).not.toBeInTheDocument();
		});
	});

	describe('outcome-only state', () => {
		it('renders a single summary row and no step list when only lastReturn is set', () => {
			const lastReturn: LastReturn = { method: 'execute', outcome: 'failed' };
			render(<StepsTimeline activeRun={null} lastReturn={lastReturn} userInstalled />);

			expect(document.querySelector('ol')).not.toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Inspect step definition' })).not.toBeInTheDocument();
		});
	});
});
