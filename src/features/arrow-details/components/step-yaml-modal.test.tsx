import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { fetchStep, runStep, signalStep } from '@/__mocks__/arrow-steps';

import { StepYamlModal, type StepYamlStep } from './step-yaml-modal';

// A `StepProgress` row (from the Activity tab) is structurally a bare
// `{ type, title }` -- core's own `StepProgressDTO` carries nothing else, so
// this is what actually reaches the modal for an already-run step.
const PROGRESS_STEP: StepYamlStep = { type: 'exec', title: 'Fetch server jar' };

describe('StepYamlModal', () => {
	it('renders the step title, an informational subtitle, and the raw type/title block', () => {
		render(<StepYamlModal onOpenChange={vi.fn()} open step={PROGRESS_STEP} />);

		expect(screen.getByRole('heading', { name: 'Fetch server jar' })).toBeInTheDocument();
		expect(screen.getByText('Raw step definition')).toBeInTheDocument();

		const pre = document.querySelector('pre');
		expect(pre).toHaveTextContent('type: exec title: Fetch server jar');
	});

	it('does not invent extra fields for a StepProgress row, which genuinely has none', () => {
		render(<StepYamlModal onOpenChange={vi.fn()} open step={PROGRESS_STEP} />);
		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe('type: exec\ntitle: Fetch server jar');
	});

	it('renders every real field of a run step: command, elevated, timeout', () => {
		const step = runStep('Start process', { command: 'java -jar server.jar', elevated: true, timeout: '45s' });
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe(
			'type: run\ntitle: Start process\ncommand: java -jar server.jar\nelevated: true\ntimeout: 45s'
		);
	});

	it('renders every real field of a fetch step: url, to, checksum, timeout', () => {
		const step = fetchStep('Fetch archive', {
			url: 'https://github.com/rabbyte/minecraft/releases/v1.21.4.tar.gz',
			to: '/opt/quiver/workdir/archive.tar.gz',
			checksum: 'sha256:8f14e45fceea167a5a36dedd4bea2543',
		});
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe(
			'type: fetch\n' +
				'title: Fetch archive\n' +
				'url: https://github.com/rabbyte/minecraft/releases/v1.21.4.tar.gz\n' +
				'to: /opt/quiver/workdir/archive.tar.gz\n' +
				'checksum: sha256:8f14e45fceea167a5a36dedd4bea2543\n' +
				'timeout: 60s'
		);
	});

	it('renders every real field of a signal step: signal, timeout', () => {
		const step = signalStep('Stop process', { signal: 'kill', timeout: '5s' });
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe('type: signal\ntitle: Stop process\nsignal: kill\ntimeout: 5s');
	});

	it('renders a dependencies step with no extra fields at all, matching core never sending exit_on_failure for it', () => {
		render(<StepYamlModal onOpenChange={vi.fn()} open step={{ type: 'dependencies', title: 'Java 21' }} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe('type: dependencies\ntitle: Java 21');
	});

	it('shows an explicit empty string as quotes, not a blank line', () => {
		const step = fetchStep('Fetch archive', { checksum: '' });
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toContain('checksum: ""');
	});

	it('renders a per-OS override object as raw JSON, not just its default', () => {
		const step = runStep('Start process', {
			command: { default: 'server', 'darwin/arm64': 'server-arm64' },
		});
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toContain('command: {"default":"server","darwin/arm64":"server-arm64"}');
	});

	it('renders exit_on_failure when the step declares it', () => {
		const step: StepYamlStep = { ...runStep('Start process'), exit_on_failure: false };
		render(<StepYamlModal onOpenChange={vi.fn()} open step={step} />);

		const pre = document.querySelector('pre');
		expect(pre?.textContent).toContain('exit_on_failure: false');
	});

	it('renders nothing when step is null', () => {
		render(<StepYamlModal onOpenChange={vi.fn()} open step={null} />);
		expect(screen.queryByText('Raw step definition')).not.toBeInTheDocument();
		expect(document.querySelector('pre')).not.toBeInTheDocument();
	});

	it('renders nothing when closed', () => {
		render(<StepYamlModal onOpenChange={vi.fn()} open={false} step={PROGRESS_STEP} />);
		expect(screen.queryByText('Fetch server jar')).not.toBeInTheDocument();
	});

	it('calls onOpenChange when dismissed', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		render(<StepYamlModal onOpenChange={onOpenChange} open step={PROGRESS_STEP} />);

		await user.click(screen.getByRole('button', { name: 'Close' }));
		expect(onOpenChange).toHaveBeenCalledWith(false, expect.anything());
	});
});
