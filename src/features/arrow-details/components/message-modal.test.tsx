import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { MessageModal } from './message-modal';

describe('MessageModal', () => {
	it('renders the title and the message, preserving its line breaks', () => {
		render(
			<MessageModal
				message={'quiver.core lost track of this process.\nexit code 137'}
				onOpenChange={vi.fn()}
				open
				title="Detached"
			/>
		);

		expect(screen.getByRole('heading', { name: 'Detached' })).toBeInTheDocument();
		const pre = document.querySelector('pre');
		expect(pre?.textContent).toBe('quiver.core lost track of this process.\nexit code 137');
	});

	it('renders nothing when closed', () => {
		render(<MessageModal message="x" onOpenChange={vi.fn()} open={false} title="Detached" />);
		expect(screen.queryByText('Detached')).not.toBeInTheDocument();
		expect(document.querySelector('pre')).not.toBeInTheDocument();
	});

	it('calls onOpenChange when dismissed via the close button', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		render(<MessageModal message="x" onOpenChange={onOpenChange} open title="Detached" />);

		await user.click(screen.getByRole('button', { name: 'Close' }));
		expect(onOpenChange).toHaveBeenCalledWith(false, expect.anything());
	});

	it('renders no footer when onConfirm is omitted', () => {
		render(<MessageModal message="x" onOpenChange={vi.fn()} open title="Detached" />);
		expect(screen.queryByRole('button', { name: /confirm|cancel|add anyway/i })).not.toBeInTheDocument();
	});

	it('renders a Cancel/Confirm footer when onConfirm is provided, and confirming calls it', async () => {
		const user = userEvent.setup();
		const onConfirm = vi.fn();
		render(
			<MessageModal
				cancelLabel="Cancel"
				confirmLabel="Add anyway"
				message="x"
				onConfirm={onConfirm}
				onOpenChange={vi.fn()}
				open
				title="Not supported"
			/>
		);

		await user.click(screen.getByRole('button', { name: 'Add anyway' }));
		expect(onConfirm).toHaveBeenCalledOnce();
	});

	it('closes without calling onConfirm when Cancel is clicked', async () => {
		const user = userEvent.setup();
		const onConfirm = vi.fn();
		const onOpenChange = vi.fn();
		render(
			<MessageModal
				cancelLabel="Cancel"
				confirmLabel="Add anyway"
				message="x"
				onConfirm={onConfirm}
				onOpenChange={onOpenChange}
				open
				title="Not supported"
			/>
		);

		await user.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onConfirm).not.toHaveBeenCalled();
		expect(onOpenChange).toHaveBeenCalledWith(false);
	});
});
