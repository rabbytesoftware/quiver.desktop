import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { runStep } from '@/__mocks__/arrow-steps';
import type { ArrowVariable } from '@/domain/arrow';
import type { ArrowAction } from '@/features/arrow-details/lib/actions';

import { ActionButton } from './action-button';

const BASE_ACTION: ArrowAction = {
	kind: 'install',
	labelKey: 'arrow.action.install',
	variant: 'default',
	steps: [runStep('Fetch archive')],
	usesVariables: [],
	forceBusy: false,
	forceDisabled: false,
};

const VARIABLES: ArrowVariable[] = [{ name: 'server-name', description: '', type: 'string', default: 'My Server' }];

function renderButton(
	action: Partial<ArrowAction> = {},
	props: Partial<React.ComponentProps<typeof ActionButton>> = {}
) {
	const onInvoke = vi.fn();
	const onValueChange = vi.fn();
	render(
		<ActionButton
			action={{ ...BASE_ACTION, ...action }}
			onInvoke={onInvoke}
			onValueChange={onValueChange}
			pending={false}
			values={{}}
			variables={VARIABLES}
			{...props}
		/>
	);
	return { onInvoke, onValueChange };
}

describe('ActionButton', () => {
	it('renders the action label and calls onInvoke when clicked', async () => {
		const user = userEvent.setup();
		const { onInvoke } = renderButton();

		const main = screen.getByRole('button', { name: 'Install' });
		expect(main).not.toBeDisabled();
		await user.click(main);
		expect(onInvoke).toHaveBeenCalledTimes(1);
	});

	it('shows the busy label and disables the button when forceBusy is true, and does not invoke on click', async () => {
		const user = userEvent.setup();
		const { onInvoke } = renderButton({ forceBusy: true, busyLabelKey: 'arrow.action.installing' });

		const main = screen.getByRole('button', { name: 'Installing…' });
		expect(main).toBeDisabled();
		await user.click(main);
		expect(onInvoke).not.toHaveBeenCalled();
	});

	it('falls back to the plain label when forceBusy is true but no busyLabelKey is given', () => {
		renderButton({ forceBusy: true });
		expect(screen.getByRole('button', { name: 'Install' })).toBeDisabled();
	});

	it('shows the plain label, disabled, when forceDisabled is true (no busy label)', () => {
		renderButton({ forceDisabled: true });
		expect(screen.getByRole('button', { name: 'Install' })).toBeDisabled();
	});

	it('treats the pending prop as busy even when the action itself is not forceBusy -- the optimistic in-flight state', () => {
		renderButton({}, { pending: true });
		expect(screen.getByRole('button', { name: 'Install' })).toBeDisabled();
	});

	it('the info trigger stays enabled and clickable even while the main action is disabled', async () => {
		const user = userEvent.setup();
		renderButton({ forceDisabled: true });

		const info = screen.getByRole('button', { name: 'What this does' });
		expect(info).not.toBeDisabled();
		await user.click(info);
		expect(await screen.findByRole('heading', { name: 'Install' })).toBeInTheDocument();
	});

	it('opens the step preview with the action label as title and its steps listed', async () => {
		const user = userEvent.setup();
		renderButton();

		await user.click(screen.getByRole('button', { name: 'What this does' }));
		expect(await screen.findByRole('heading', { name: 'Install' })).toBeInTheDocument();
		expect(screen.getByText('Fetch archive')).toBeInTheDocument();
	});

	it('passes usesVariables/variables/values/onValueChange through to the step preview', async () => {
		const user = userEvent.setup();
		const { onValueChange } = renderButton(
			{ usesVariables: ['server-name'] },
			{ values: { 'server-name': 'My Server' } }
		);

		await user.click(screen.getByRole('button', { name: 'What this does' }));
		await user.click(await screen.findByRole('button', { name: 'Configure' }));
		const field = await screen.findByRole('textbox', { name: 'server-name' });
		await user.type(field, '!');
		expect(onValueChange).toHaveBeenCalledWith('server-name', 'My Server!');
	});

	it('renders no icon for an action kind with no mapped icon (e.g. removeFromLibrary)', () => {
		renderButton({ kind: 'removeFromLibrary', labelKey: 'arrow.action.removeFromLibrary' });
		const main = screen.getByRole('button', { name: 'Remove from Library' });
		expect(main.querySelector('svg')).not.toBeInTheDocument();
	});

	it('renders a mapped icon for an action kind that has one', () => {
		renderButton({ kind: 'execute', labelKey: 'arrow.action.start' });
		const main = screen.getByRole('button', { name: 'Start' });
		expect(main.querySelector('svg')).toBeInTheDocument();
	});

	it('shows the flicker spinner in place of the icon while busy', () => {
		renderButton({
			kind: 'execute',
			labelKey: 'arrow.action.start',
			forceBusy: true,
			busyLabelKey: 'arrow.action.installing',
		});
		const main = screen.getByRole('button', { name: 'Installing…' });
		expect(main.querySelector('[data-slot="flicker-spinner"]')).toBeInTheDocument();
	});

	it('renders every declared variant as a single fused container without crashing', () => {
		const { rerender } = render(<div />);
		for (const variant of ['default', 'outline', 'destructive', 'destructive-outline'] as const) {
			rerender(
				<ActionButton
					action={{ ...BASE_ACTION, variant }}
					onInvoke={vi.fn()}
					onValueChange={vi.fn()}
					pending={false}
					values={{}}
					variables={[]}
				/>
			);
			expect(screen.getByRole('button', { name: 'Install' })).toBeInTheDocument();
		}
	});

	it('hides the info trigger and divider entirely when the action has no steps to preview (e.g. Add to Library)', () => {
		renderButton({ kind: 'addToLibrary', labelKey: 'arrow.action.addToLibrary', steps: [] });
		expect(screen.getByRole('button', { name: 'Add to Library' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'What this does' })).not.toBeInTheDocument();
	});
});
