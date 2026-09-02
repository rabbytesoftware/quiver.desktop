import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { RenameConnectionDialog } from './rename-connection-dialog';

function noop() {}

describe('RenameConnectionDialog', () => {
	it('renders nothing when closed', () => {
		render(<RenameConnectionDialog initialName="Home Lab" onOpenChange={noop} onSubmit={noop} open={false} />);
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('pre-fills the field with the current name', () => {
		render(<RenameConnectionDialog initialName="Home Lab" onOpenChange={noop} onSubmit={noop} open />);
		expect(screen.getByLabelText('Name')).toHaveValue('Home Lab');
	});

	it('disables Save once the field is emptied', async () => {
		const user = userEvent.setup();
		render(<RenameConnectionDialog initialName="Home Lab" onOpenChange={noop} onSubmit={noop} open />);
		await user.clear(screen.getByLabelText('Name'));
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('submits the trimmed new name', async () => {
		const user = userEvent.setup();
		const onSubmit = vi.fn();
		render(<RenameConnectionDialog initialName="Home Lab" onOpenChange={noop} onSubmit={onSubmit} open />);
		await user.clear(screen.getByLabelText('Name'));
		await user.type(screen.getByLabelText('Name'), '  Staging  ');
		await user.click(screen.getByRole('button', { name: 'Save' }));
		expect(onSubmit).toHaveBeenCalledWith('Staging');
	});

	it('calls onOpenChange(false) when Cancel is clicked', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		render(<RenameConnectionDialog initialName="Home Lab" onOpenChange={onOpenChange} onSubmit={noop} open />);
		await user.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onOpenChange).toHaveBeenCalledWith(false);
	});
});
