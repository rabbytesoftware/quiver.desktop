import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { fetchStep, runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowStepDefinition, ArrowVariable } from '@/domain/arrow';

import { StepPreviewModal } from './step-preview-modal';

const STEPS: ArrowStepDefinition[] = [
	{ type: 'dependencies', title: 'Java 21' },
	fetchStep('Fetch server jar'),
	runStep('Run installer'),
	signalStep('Wait for ready'),
];

const VARIABLES: ArrowVariable[] = [
	{ name: 'server-name', description: 'Shown in the server list.', type: 'string', default: 'My Server' },
	{ name: 'difficulty', description: 'How hard the world is.', type: 'string', default: 'normal' },
];

describe('StepPreviewModal', () => {
	it('renders the title, the read-only subtitle, and every step with its kind label', () => {
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" />);

		expect(screen.getByRole('heading', { name: 'Install' })).toBeInTheDocument();
		expect(screen.getByText('What runs when you choose this')).toBeInTheDocument();

		expect(screen.getByText('Java 21')).toBeInTheDocument();
		expect(screen.getByText('deps')).toBeInTheDocument();
		expect(screen.getByText('Fetch server jar')).toBeInTheDocument();
		expect(screen.getByText('fetch')).toBeInTheDocument();
		expect(screen.getByText('Run installer')).toBeInTheDocument();
		expect(screen.getByText('run')).toBeInTheDocument();
		expect(screen.getByText('Wait for ready')).toBeInTheDocument();
		expect(screen.getByText('signal')).toBeInTheDocument();
	});

	it('renders nothing when closed', () => {
		render(<StepPreviewModal onOpenChange={vi.fn()} open={false} steps={STEPS} title="Install" />);
		expect(screen.queryByText('Install')).not.toBeInTheDocument();
		expect(screen.queryByText('Run installer')).not.toBeInTheDocument();
	});

	it('omits the Uses line and the Configure link when usesVariables is not given', () => {
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" />);
		expect(screen.queryByText('Uses')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Configure' })).not.toBeInTheDocument();
	});

	it('omits the Uses line when usesVariables is given but empty', () => {
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" usesVariables={[]} />);
		expect(screen.queryByText('Uses')).not.toBeInTheDocument();
	});

	it('shows the Uses line with every consumed variable name, plus a Configure link', () => {
		render(
			<StepPreviewModal
				onOpenChange={vi.fn()}
				open
				steps={STEPS}
				title="Install"
				usesVariables={['server-name', 'difficulty']}
				variables={VARIABLES}
			/>
		);
		expect(screen.getByText('Uses').closest('p')).toHaveTextContent('Uses: server-name, difficulty');
		expect(screen.getByRole('button', { name: 'Configure' })).toBeInTheDocument();
	});

	it('opens the shared variables dialog from the Configure link, wired to the passed-through values/onValueChange', async () => {
		const user = userEvent.setup();
		const onValueChange = vi.fn();
		render(
			<StepPreviewModal
				onOpenChange={vi.fn()}
				onValueChange={onValueChange}
				open
				steps={STEPS}
				title="Install"
				usesVariables={['server-name']}
				values={{ 'server-name': 'My Server' }}
				variables={VARIABLES}
			/>
		);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();

		const field = screen.getByRole('textbox', { name: 'server-name' });
		await user.type(field, '!');
		expect(onValueChange).toHaveBeenCalledWith('server-name', 'My Server!');
	});

	it('defaults variables and values to empty, and onValueChange to a no-op, when they are not provided', async () => {
		const user = userEvent.setup();
		render(
			<StepPreviewModal
				onOpenChange={vi.fn()}
				open
				steps={STEPS}
				title="Install"
				usesVariables={['hardcore']}
				variables={[{ name: 'hardcore', description: '', type: 'boolean', default: 'false' }]}
			/>
		);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		const toggle = await screen.findByRole('switch', { name: 'hardcore' });

		// No onValueChange was passed, so the fallback no-op absorbs this --
		// the assertion is simply that clicking does not throw.
		await user.click(toggle);
		expect(toggle).toBeInTheDocument();
	});

	it('falls back to an empty variables list and values map when neither is provided', async () => {
		const user = userEvent.setup();
		render(
			<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" usesVariables={['ghost-var']} />
		);

		await user.click(screen.getByRole('button', { name: 'Configure' }));
		expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
		expect(screen.queryByRole('switch')).not.toBeInTheDocument();
	});

	it('calls onOpenChange when the dialog is dismissed', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		render(<StepPreviewModal onOpenChange={onOpenChange} open steps={STEPS} title="Install" />);

		await user.click(screen.getByRole('button', { name: 'Close' }));
		expect(onOpenChange).toHaveBeenCalledWith(false, expect.anything());
	});

	it('opens the raw step definition for the inspected step, with its real fields (not just type/title)', async () => {
		const user = userEvent.setup();
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" />);

		const row = screen.getByText('Run installer').closest('li')!;
		await user.click(within(row).getByRole('button', { name: 'Inspect step definition' }));

		expect(await screen.findByText('Raw step definition')).toBeInTheDocument();
		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe(
			'type: run\ntitle: Run installer\ncommand: echo hello\nelevated: false\ntimeout: 30s'
		);
	});

	it('inspects a different step correctly when a different row is clicked', async () => {
		const user = userEvent.setup();
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" />);

		const row = screen.getByText('Fetch server jar').closest('li')!;
		await user.click(within(row).getByRole('button', { name: 'Inspect step definition' }));

		expect(await screen.findByRole('heading', { name: 'Fetch server jar' })).toBeInTheDocument();
		const pre = document.querySelector('pre');
		expect(pre?.textContent).toContain('type: fetch');
		expect(pre?.textContent).toContain('url: https://example.com/archive.tar.gz');
	});

	it('closes the raw step definition without closing the preview itself', async () => {
		const user = userEvent.setup();
		render(<StepPreviewModal onOpenChange={vi.fn()} open steps={STEPS} title="Install" />);

		const row = screen.getByText('Run installer').closest('li')!;
		await user.click(within(row).getByRole('button', { name: 'Inspect step definition' }));
		expect(await screen.findByText('Raw step definition')).toBeInTheDocument();

		// Escape closes only the topmost (most-recently-opened) dialog.
		await user.keyboard('{Escape}');
		expect(screen.queryByText('Raw step definition')).not.toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Install' })).toBeInTheDocument();
	});
});
