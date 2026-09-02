import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

import { CommandPalette } from './command-palette';
import { useRemoteStore } from '../stores/remote-store';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

function renderPalette() {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return render(createElement(QueryClientProvider, { client: qc }, createElement(CommandPalette)));
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

describe('CommandPalette', () => {
	it('renders nothing when closed', () => {
		renderPalette();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('lists every connection and marks the active one Current, when open', () => {
		useRemoteStore.getState().openCmd();
		renderPalette();
		expect(screen.getByText('Local')).toBeInTheDocument();
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
		expect(screen.getByText('Current')).toBeInTheDocument();
	});

	it('filters the list as the query changes', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.type(screen.getByRole('textbox'), 'home');

		expect(screen.queryByText('Local')).not.toBeInTheDocument();
		expect(screen.getByText('Home Lab')).toBeInTheDocument();
	});

	it('shows an empty message when nothing matches', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.type(screen.getByRole('textbox'), 'garage');

		expect(screen.getByText('No connections match "garage".')).toBeInTheDocument();
	});

	it('Escape closes the palette', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.type(screen.getByRole('textbox'), '{Escape}');

		expect(useRemoteStore.getState().cmdOpen).toBe(false);
	});

	it('Enter switches to the highlighted connection and closes', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.type(screen.getByRole('textbox'), 'home{Enter}');

		await waitFor(() => expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'home-lab' }));
		expect(useRemoteStore.getState().cmdOpen).toBe(false);
	});

	it('clicking an item switches to it', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.click(screen.getByText('Home Lab'));

		await waitFor(() => expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'home-lab' }));
	});

	it('clicking the already-active, healthy connection just closes without switching', async () => {
		const user = userEvent.setup();
		useRemoteStore.getState().openCmd();
		renderPalette();

		await user.click(screen.getByText('Local'));

		expect(invoke).not.toHaveBeenCalled();
		expect(useRemoteStore.getState().cmdOpen).toBe(false);
	});
});
