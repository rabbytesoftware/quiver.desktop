import { createElement } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { useAddConnection, useRemoveConnection, useSwitchConnection, useRenameConnection } from './connection';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as MockedFunction<typeof invoke>;

function makeWrapper() {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const Wrapper = ({ children }: { children: React.ReactNode }) =>
		createElement(QueryClientProvider, { client: qc }, children);
	return Wrapper;
}

beforeEach(() => {
	mockInvoke.mockResolvedValue(undefined as never);
});

describe('useAddConnection', () => {
	it('calls add_connection with name, url, token', async () => {
		const { result } = renderHook(() => useAddConnection(), { wrapper: makeWrapper() });
		await act(() => result.current.mutateAsync({ name: 'Prod', url: 'tcp://10.0.0.1:40257', token: 'tok' }));
		expect(invoke).toHaveBeenCalledWith('add_connection', {
			name: 'Prod',
			url: 'tcp://10.0.0.1:40257',
			token: 'tok',
		});
	});
});

describe('useRemoveConnection', () => {
	it('calls remove_connection with id', async () => {
		const { result } = renderHook(() => useRemoveConnection(), { wrapper: makeWrapper() });
		await act(() => result.current.mutateAsync({ id: 'abc-123' }));
		expect(invoke).toHaveBeenCalledWith('remove_connection', { id: 'abc-123' });
	});
});

describe('useSwitchConnection', () => {
	it('calls switch_connection with id', async () => {
		const { result } = renderHook(() => useSwitchConnection(), { wrapper: makeWrapper() });
		await act(() => result.current.mutateAsync({ id: 'abc-123' }));
		expect(invoke).toHaveBeenCalledWith('switch_connection', { id: 'abc-123' });
	});
});

describe('useRenameConnection', () => {
	it('calls rename_connection with id and name', async () => {
		const { result } = renderHook(() => useRenameConnection(), { wrapper: makeWrapper() });
		await act(() => result.current.mutateAsync({ id: 'abc-123', name: 'Staging' }));
		expect(invoke).toHaveBeenCalledWith('rename_connection', { id: 'abc-123', name: 'Staging' });
	});
});
