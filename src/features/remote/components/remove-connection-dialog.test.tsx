import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { RemoveConnectionDialog } from './remove-connection-dialog';

function noop() {}

describe('RemoveConnectionDialog', () => {
	it('renders nothing when closed', () => {
		render(
			<RemoveConnectionDialog
				isActive={false}
				name="Home Lab"
				onConfirm={noop}
				onOpenChange={noop}
				open={false}
			/>
		);
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('names the connection in the title', () => {
		render(<RemoveConnectionDialog isActive={false} name="Home Lab" onConfirm={noop} onOpenChange={noop} open />);
		expect(screen.getByText('Remove Home Lab?')).toBeInTheDocument();
	});

	it('shows the active-connection warning only when removing the active one', () => {
		const { rerender } = render(
			<RemoveConnectionDialog isActive={false} name="Home Lab" onConfirm={noop} onOpenChange={noop} open />
		);
		expect(screen.queryByText(/switches you back to Local/)).not.toBeInTheDocument();

		rerender(<RemoveConnectionDialog isActive name="Home Lab" onConfirm={noop} onOpenChange={noop} open />);
		expect(screen.getByText(/switches you back to Local/)).toBeInTheDocument();
	});

	it('calls onConfirm when Remove is clicked', async () => {
		const user = userEvent.setup();
		const onConfirm = vi.fn();
		render(
			<RemoveConnectionDialog isActive={false} name="Home Lab" onConfirm={onConfirm} onOpenChange={noop} open />
		);
		await user.click(screen.getByRole('button', { name: 'Remove' }));
		expect(onConfirm).toHaveBeenCalledOnce();
	});

	it('calls onOpenChange(false) when Cancel is clicked', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		render(
			<RemoveConnectionDialog
				isActive={false}
				name="Home Lab"
				onConfirm={noop}
				onOpenChange={onOpenChange}
				open
			/>
		);
		await user.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onOpenChange).toHaveBeenCalledWith(false);
	});
});
