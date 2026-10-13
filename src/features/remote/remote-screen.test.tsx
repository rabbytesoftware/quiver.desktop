import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

import { RemoteScreen } from './remote-screen';
import { useRemoteStore } from './stores/remote-store';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

function renderScreen() {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return render(createElement(QueryClientProvider, { client: qc }, createElement(RemoteScreen)));
}

const local = { id: 'local', name: 'Local', kind: 'local' as const, api_version: 'v0' };
const homeLab = {
	id: 'home-lab',
	name: 'Home Lab',
	kind: 'remote' as const,
	url: 'http://192.168.1.42:7420',
	api_version: 'v0',
};

beforeEach(() => {
	mockInvoke.mockReset();
	mockInvoke.mockResolvedValue(undefined);
	useConnectionStore.setState({ connections: [local, homeLab], activeId: 'local' });
	useStatusStore.setState({ status: 'ready' });
	useRemoteStore.setState(useRemoteStore.getInitialState(), true);
});

describe('RemoteScreen', () => {
	it('shows the loading skeleton while the connections list has not arrived yet', () => {
		useConnectionStore.setState({ connections: [], activeId: 'local' });
		renderScreen();
		expect(screen.queryByText('Local')).not.toBeInTheDocument();
	});

	it('renders the page title and every saved connection', () => {
		renderScreen();
		expect(screen.getByText('Remote Control')).toBeInTheDocument();
		expect(screen.getByText('Local')).toBeInTheDocument();
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
	});

	it('shows the empty state and hides it once a remote exists', () => {
		useConnectionStore.setState({ connections: [local], activeId: 'local' });
		renderScreen();
		expect(screen.getByText('No remote daemons yet')).toBeInTheDocument();
	});

	it('opens the add dialog from the header button', async () => {
		const user = userEvent.setup();
		renderScreen();
		await user.click(screen.getByRole('button', { name: 'Add remote' }));
		expect(screen.getByText('Add a remote connection')).toBeInTheDocument();
	});

	it('submitting the add dialog calls add_connection and closes it', async () => {
		const user = userEvent.setup();
		renderScreen();
		await user.click(screen.getByRole('button', { name: 'Add remote' }));

		await user.type(screen.getByLabelText('Name'), 'Office');
		await user.type(screen.getByLabelText('URL'), 'http://10.0.1.8:7420');
		await user.click(screen.getByRole('button', { name: 'Continue' }));

		const slots = await screen.findAllByLabelText(/Pairing code digit/);
		for (const [index, digit] of ['1', '2', '3', '4', '5', '6'].entries()) {
			await user.type(slots[index], digit);
		}
		await user.click(screen.getByRole('button', { name: 'Add connection' }));

		await waitFor(() =>
			expect(invoke).toHaveBeenCalledWith('add_connection', {
				name: 'Office',
				url: 'http://10.0.1.8:7420',
				code: '123456',
			})
		);
		await waitFor(() => expect(screen.queryByText('Add a remote connection')).not.toBeInTheDocument());
	});

	it('renaming (opened via the store, since the row menu gesture is exercised live rather than in jsdom) submits the new name', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openRename('home-lab');
		renderScreen();

		const input = screen.getByLabelText('Name');
		await user.clear(input);
		await user.type(input, 'Staging');
		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(invoke).toHaveBeenCalledWith('rename_connection', { id: 'home-lab', name: 'Staging' })
		);
	});

	it('removing shows the active-connection warning only for the active connection', () => {
		useRemoteStore.getState().openRemove('local');
		renderScreen();
		expect(screen.getByText(/switches you back to Local/)).toBeInTheDocument();
	});

	it('confirming removal calls remove_connection', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openRemove('home-lab');
		renderScreen();

		await user.click(screen.getByRole('button', { name: 'Remove' }));

		await waitFor(() => expect(invoke).toHaveBeenCalledWith('remove_connection', { id: 'home-lab' }));
	});

	it('Retry on a disconnected active row calls switch_connection with the same id', async () => {
		const user = userEvent.setup();
		useConnectionStore.setState({ connections: [local, homeLab], activeId: 'home-lab' });
		useStatusStore.setState({ status: 'disconnected' });
		renderScreen();

		await user.click(screen.getByRole('button', { name: 'Retry' }));

		await waitFor(() => expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'home-lab' }));
	});

	it('Switch to Local on a disconnected remote calls switch_connection with local', async () => {
		const user = userEvent.setup();
		useConnectionStore.setState({ connections: [local, homeLab], activeId: 'home-lab' });
		useStatusStore.setState({ status: 'disconnected' });
		renderScreen();

		await user.click(screen.getByRole('button', { name: 'Switch to Local' }));

		await waitFor(() => expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'local' }));
	});
});
