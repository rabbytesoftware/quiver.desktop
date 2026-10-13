import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { AddConnectionDialog } from './add-connection-dialog';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

function noop() {}

function renderDialog(props: Partial<Parameters<typeof AddConnectionDialog>[0]> = {}) {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
	return render(
		createElement(
			QueryClientProvider,
			{ client: qc },
			createElement(AddConnectionDialog, {
				busy: false,
				onOpenChange: noop,
				onSubmit: noop,
				open: true,
				...props,
			})
		)
	);
}

async function fillDetails(
	user: ReturnType<typeof userEvent.setup>,
	name = 'Home Lab',
	url = 'http://192.168.1.42:7420'
) {
	await user.type(screen.getByLabelText('Name'), name);
	await user.type(screen.getByLabelText('URL'), url);
}

beforeEach(() => {
	mockInvoke.mockReset();
});

describe('AddConnectionDialog', () => {
	it('renders nothing when closed', () => {
		renderDialog({ open: false });
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('starts on the details stage, with Continue disabled until Name and URL are filled', async () => {
		const user = userEvent.setup();
		renderDialog();

		expect(screen.queryByLabelText(/Pairing code digit/)).not.toBeInTheDocument();
		const cont = screen.getByRole('button', { name: 'Continue' });
		expect(cont).toBeDisabled();

		await fillDetails(user);
		expect(cont).toBeEnabled();
	});

	it('advances to the pairing stage once the health check succeeds', async () => {
		const user = userEvent.setup();
		mockInvoke.mockResolvedValue(undefined);
		renderDialog();

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));

		expect(invoke).toHaveBeenCalledWith('check_remote_health', { url: 'http://192.168.1.42:7420' });
		await waitFor(() => expect(screen.getAllByLabelText(/Pairing code digit/)).toHaveLength(6));
		expect(screen.queryByLabelText('Name')).not.toBeInTheDocument();
	});

	it('shows an error and stays on the details stage when the health check fails', async () => {
		const user = userEvent.setup();
		mockInvoke.mockRejectedValue(new Error('unreachable'));
		renderDialog();

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));

		expect(await screen.findByText(/Couldn't reach that address/)).toBeInTheDocument();
		expect(screen.queryByLabelText(/Pairing code digit/)).not.toBeInTheDocument();
	});

	it('submits name, url, and the joined pairing code from the pairing stage', async () => {
		const user = userEvent.setup();
		mockInvoke.mockResolvedValue(undefined);
		const onSubmit = vi.fn();
		renderDialog({ onSubmit });

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));
		const slots = await screen.findAllByLabelText(/Pairing code digit/);
		for (const [index, digit] of ['4', '8', '2', '9', '1', '3'].entries()) {
			await user.type(slots[index], digit);
		}
		await user.click(screen.getByRole('button', { name: 'Add connection' }));

		expect(onSubmit).toHaveBeenCalledWith({ name: 'Home Lab', url: 'http://192.168.1.42:7420', code: '482913' });
	});

	it('Back returns to the details stage without losing what was typed', async () => {
		const user = userEvent.setup();
		mockInvoke.mockResolvedValue(undefined);
		renderDialog();

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));
		await screen.findAllByLabelText(/Pairing code digit/);

		await user.click(screen.getByRole('button', { name: 'Back' }));

		expect(screen.getByLabelText('Name')).toHaveValue('Home Lab');
		expect(screen.getByLabelText('URL')).toHaveValue('http://192.168.1.42:7420');
	});

	it('calls onOpenChange(false) when Cancel is clicked', async () => {
		const user = userEvent.setup();
		const onOpenChange = vi.fn();
		renderDialog({ onOpenChange });
		await user.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onOpenChange).toHaveBeenCalledWith(false);
	});

	it('resets to the details stage and clears every field when reopened', async () => {
		const user = userEvent.setup();
		mockInvoke.mockResolvedValue(undefined);
		const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
		const props = { busy: false, onOpenChange: noop, onSubmit: noop };
		const { rerender } = render(
			createElement(
				QueryClientProvider,
				{ client: qc },
				createElement(AddConnectionDialog, { ...props, open: true })
			)
		);

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));
		await screen.findAllByLabelText(/Pairing code digit/);

		rerender(
			createElement(
				QueryClientProvider,
				{ client: qc },
				createElement(AddConnectionDialog, { ...props, open: false })
			)
		);
		rerender(
			createElement(
				QueryClientProvider,
				{ client: qc },
				createElement(AddConnectionDialog, { ...props, open: true })
			)
		);

		expect(screen.getByLabelText('Name')).toHaveValue('');
		expect(screen.getByLabelText('URL')).toHaveValue('');
		expect(screen.queryByLabelText(/Pairing code digit/)).not.toBeInTheDocument();
	});

	it('shows a busy label and disables submit on the pairing stage while adding', async () => {
		const user = userEvent.setup();
		mockInvoke.mockResolvedValue(undefined);
		renderDialog({ busy: true });

		await fillDetails(user);
		await user.click(screen.getByRole('button', { name: 'Continue' }));
		await screen.findAllByLabelText(/Pairing code digit/);

		expect(screen.getByRole('button', { name: /Adding…/ })).toBeDisabled();
	});
});
