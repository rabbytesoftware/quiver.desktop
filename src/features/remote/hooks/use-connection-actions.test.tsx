import { createElement, type ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

import { useConnectionActions } from './use-connection-actions';
import { useRemoteStore } from '../stores/remote-store';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

function wrapper({ children }: { children: ReactNode }) {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return createElement(QueryClientProvider, { client: qc }, children);
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
	useConnectionStore.setState({ connections: [local, homeLab], activeId: 'local' });
	useStatusStore.setState({ status: 'ready' });
	useRemoteStore.setState(useRemoteStore.getInitialState(), true);
});

describe('connect', () => {
	it('optimistically moves activeId to the switched-to connection on success', async () => {
		mockInvoke.mockResolvedValue(undefined);
		useStatusStore.setState({ status: 'ready' });
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.connect('home-lab', 'Home Lab'));

		expect(useConnectionStore.getState().activeId).toBe('home-lab');
	});

	it('pushes a "Connected to" toast when the resulting status is ready', async () => {
		mockInvoke.mockImplementation(async () => {
			useStatusStore.setState({ status: 'ready' });
		});
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.connect('home-lab', 'Home Lab'));

		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(['Connected to Home Lab'])
		);
	});

	it('pushes a "Couldn\'t reach" toast when the resulting status is disconnected', async () => {
		mockInvoke.mockImplementation(async () => {
			useStatusStore.setState({ status: 'disconnected' });
		});
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.connect('garage', 'Garage Server'));

		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(["Couldn't reach Garage Server"])
		);
	});

	it('calls switch_connection with the given id', async () => {
		mockInvoke.mockResolvedValue(undefined);
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.connect('home-lab', 'Home Lab'));

		expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'home-lab' });
	});

	it('toasts a failure and leaves activeId alone when the command itself rejects', async () => {
		mockInvoke.mockRejectedValue(new Error('no such connection'));
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.connect('garage', 'Garage Server'));

		expect(useConnectionStore.getState().activeId).toBe('local');
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(["Couldn't switch to Garage Server"])
		);
	});
});

describe('addRemote', () => {
	it('adds the connection, closes the dialog, and toasts on success', async () => {
		mockInvoke.mockResolvedValue({ id: 'r1', name: 'Home Lab', kind: 'remote', api_version: 'v0' });
		useRemoteStore.getState().openAdd();
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() =>
			result.current.addRemote({ name: 'Home Lab', url: 'http://192.168.1.42:7420', code: '482913' })
		);

		expect(invoke).toHaveBeenCalledWith('add_connection', {
			name: 'Home Lab',
			url: 'http://192.168.1.42:7420',
			code: '482913',
		});
		await waitFor(() => expect(useRemoteStore.getState().addOpen).toBe(false));
		await waitFor(() => expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(['Added Home Lab']));
	});

	it('keeps the dialog open and toasts a failure when the command rejects', async () => {
		mockInvoke.mockRejectedValue(new Error('invalid pairing code'));
		useRemoteStore.getState().openAdd();
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() =>
			result.current.addRemote({ name: 'Home Lab', url: 'http://192.168.1.42:7420', code: '000000' })
		);

		expect(useRemoteStore.getState().addOpen).toBe(true);
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(["Couldn't add Home Lab"])
		);
	});
});

describe('renameConnection', () => {
	it('renames, closes the dialog, and toasts on success', async () => {
		mockInvoke.mockResolvedValue(undefined);
		useRemoteStore.getState().openRename('home-lab');
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.renameConnection('home-lab', 'Staging'));

		expect(invoke).toHaveBeenCalledWith('rename_connection', { id: 'home-lab', name: 'Staging' });
		await waitFor(() => expect(useRemoteStore.getState().renameId).toBeNull());
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(['Renamed to Staging'])
		);
	});

	it('keeps the dialog open and toasts a failure when the command rejects', async () => {
		mockInvoke.mockRejectedValue(new Error('connection not found'));
		useRemoteStore.getState().openRename('home-lab');
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.renameConnection('home-lab', 'Staging'));

		expect(useRemoteStore.getState().renameId).toBe('home-lab');
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(["Couldn't rename to Staging"])
		);
	});
});

describe('removeConnection', () => {
	it('removes, closes the dialog, and toasts on success', async () => {
		mockInvoke.mockResolvedValue(undefined);
		useRemoteStore.getState().openRemove('home-lab');
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.removeConnection('home-lab', 'Home Lab'));

		expect(invoke).toHaveBeenCalledWith('remove_connection', { id: 'home-lab' });
		await waitFor(() => expect(useRemoteStore.getState().removeId).toBeNull());
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(['Removed Home Lab'])
		);
	});

	it('keeps the dialog open and toasts a failure when the command rejects', async () => {
		mockInvoke.mockRejectedValue(new Error('connection not found'));
		useRemoteStore.getState().openRemove('home-lab');
		const { result } = renderHook(() => useConnectionActions(), { wrapper });

		await act(() => result.current.removeConnection('home-lab', 'Home Lab'));

		expect(useRemoteStore.getState().removeId).toBe('home-lab');
		await waitFor(() =>
			expect(useRemoteStore.getState().toasts.map((t) => t.message)).toEqual(["Couldn't remove Home Lab"])
		);
	});
});
