import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowLifecycle, ArrowMethod, ArrowVariable } from '@/domain/arrow';

import { MethodsPanel } from './methods-panel';

const LIFECYCLE: ArrowLifecycle = {
	install: [runStep('Fetch archive')],
	update: [],
	execute: [runStep('Start process')],
	stop: [],
	uninstall: [runStep('Remove workdir')],
};

const METHODS: ArrowMethod[] = [
	{
		available_in: ['ready', 'running'],
		description: 'Snapshots the world to disk.',
		name: 'backup',
		steps: [signalStep('Pause writes'), runStep('Copy world folder')],
	},
	{
		available_in: ['running'],
		description: '',
		name: 'rcon',
		steps: [runStep('Open console')],
	},
];

const VARIABLES: ArrowVariable[] = [
	{ default: 'world', description: 'World name', name: 'world-name', type: 'string' },
];

describe('MethodsPanel', () => {
	it('renders the Methods title with a count badge', () => {
		render(<MethodsPanel methods={METHODS} variables={[]} />);
		expect(screen.getByText('Methods')).toBeInTheDocument();
		expect(screen.getByText('2 methods')).toBeInTheDocument();
	});

	it('renders every method row with its name, description, and step count', () => {
		render(<MethodsPanel methods={METHODS} variables={[]} />);

		expect(screen.getByText('backup')).toBeInTheDocument();
		expect(screen.getByText('Snapshots the world to disk.')).toBeInTheDocument();
		expect(screen.getByText('2 steps')).toBeInTheDocument();

		expect(screen.getByText('rcon')).toBeInTheDocument();
		expect(screen.getByText('1 steps')).toBeInTheDocument();
	});

	it('omits the description paragraph for a method with an empty description', () => {
		render(<MethodsPanel methods={[METHODS[1]]} variables={[]} />);
		expect(screen.queryByText('Snapshots the world to disk.')).not.toBeInTheDocument();
		expect(screen.getByText('1 steps')).toBeInTheDocument();
	});

	it('renders no rows and a 0-count badge when there are no methods', () => {
		render(<MethodsPanel methods={[]} variables={[]} />);
		expect(screen.getByText('0 methods')).toBeInTheDocument();
	});

	it('opens the preview modal for the clicked method, without a Uses line, and closes it again', async () => {
		const user = userEvent.setup();
		const onValueChange = vi.fn();
		render(
			<MethodsPanel
				methods={METHODS}
				onValueChange={onValueChange}
				values={{ 'world-name': 'world' }}
				variables={VARIABLES}
			/>
		);

		const infoButtons = screen.getAllByRole('button', { name: 'What this does' });
		await user.click(infoButtons[0]);

		expect(await screen.findByText('What runs when you choose this')).toBeInTheDocument();
		expect(screen.getByText('Pause writes')).toBeInTheDocument();
		expect(screen.getByText('Copy world folder')).toBeInTheDocument();

		// No per-method variable scoping exists in the domain model, so the
		// "Uses" line -- and the Configure link that comes with it -- never
		// renders for a custom method's preview, even though variables were
		// passed through for the Configure flow other panels use.
		expect(screen.queryByText('Uses')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Configure' })).not.toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'Close' }));
		await waitFor(() => expect(screen.queryByText('What runs when you choose this')).not.toBeInTheDocument());
	});

	it('opens the correct method when a different row’s info button is clicked', async () => {
		const user = userEvent.setup();
		render(<MethodsPanel methods={METHODS} variables={[]} />);

		const infoButtons = screen.getAllByRole('button', { name: 'What this does' });
		await user.click(infoButtons[1]);

		expect(await screen.findByText('Open console')).toBeInTheDocument();
		expect(screen.queryByText('Pause writes')).not.toBeInTheDocument();
	});

	it('is read-only: renders no Run affordance for any method', () => {
		render(<MethodsPanel methods={METHODS} variables={[]} />);
		expect(screen.queryByRole('button', { name: /run/i })).not.toBeInTheDocument();
	});

	it('lists lifecycle actions before custom methods, in install/update/execute/stop/uninstall order, skipping ones with no steps', () => {
		const { container } = render(<MethodsPanel lifecycle={LIFECYCLE} methods={METHODS} variables={[]} />);

		const names = [...container.querySelectorAll('p.font-mono')].map((el) => el.textContent);
		expect(names).toEqual(['Install', 'Start', 'Uninstall', 'backup', 'rcon']);
	});

	it('counts lifecycle actions and custom methods together in the count badge', () => {
		render(<MethodsPanel lifecycle={LIFECYCLE} methods={METHODS} variables={[]} />);
		expect(screen.getByText('5 methods')).toBeInTheDocument();
	});

	it('shows a lifecycle action’s step count without a description paragraph', () => {
		const onlyInstall: ArrowLifecycle = { ...LIFECYCLE, execute: [], uninstall: [] };
		render(<MethodsPanel lifecycle={onlyInstall} methods={[]} variables={[]} />);

		expect(screen.getByText('Install')).toBeInTheDocument();
		expect(screen.getByText('1 steps')).toBeInTheDocument();
	});

	it('previews a lifecycle action’s own steps when its info button is clicked', async () => {
		const user = userEvent.setup();
		render(<MethodsPanel lifecycle={LIFECYCLE} methods={[]} variables={[]} />);

		const infoButtons = screen.getAllByRole('button', { name: 'What this does' });
		await user.click(infoButtons[0]);

		expect(await screen.findByText('Fetch archive')).toBeInTheDocument();
	});

	it('renders only custom methods when lifecycle is omitted, unchanged from before', () => {
		render(<MethodsPanel methods={METHODS} variables={[]} />);
		expect(screen.getByText('2 methods')).toBeInTheDocument();
		expect(screen.queryByText('Install')).not.toBeInTheDocument();
	});
});
