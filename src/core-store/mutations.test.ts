import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    useInstall, useUninstall, useExecute, useStop,
    useRegisterArrow, useRemoveArrow, useFollowCollection, useUnfollowCollection,
} from './mutations';

vi.mock('@tauri-apps/api/core');

function wrapper() {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    return ({ children }: { children: React.ReactNode }) =>
        createElement(QueryClientProvider, { client: qc }, children);
}

beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue(undefined);
});

describe('useInstall', () => {
    it('calls invoke with install command and namespace', async () => {
        const { result } = renderHook(() => useInstall(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
        expect(invoke).toHaveBeenCalledWith('install', { namespace: 'ns@v1', variables: {} });
    });

    it('passes variables when provided', async () => {
        const { result } = renderHook(() => useInstall(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1', variables: { KEY: 'val' } }));
        expect(invoke).toHaveBeenCalledWith('install', { namespace: 'ns@v1', variables: { KEY: 'val' } });
    });
});

describe('useUninstall', () => {
    it('calls invoke with uninstall command', async () => {
        const { result } = renderHook(() => useUninstall(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
        expect(invoke).toHaveBeenCalledWith('uninstall', { namespace: 'ns@v1', variables: {} });
    });
});

describe('useExecute', () => {
    it('calls invoke with execute command and method', async () => {
        const { result } = renderHook(() => useExecute(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1', method: '_execute' }));
        expect(invoke).toHaveBeenCalledWith('execute', { namespace: 'ns@v1', method: '_execute', variables: {} });
    });
});

describe('useStop', () => {
    it('calls invoke with stop command', async () => {
        const { result } = renderHook(() => useStop(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
        expect(invoke).toHaveBeenCalledWith('stop', { namespace: 'ns@v1' });
    });
});

describe('useRegisterArrow', () => {
    it('calls invoke with register_arrow command', async () => {
        const { result } = renderHook(() => useRegisterArrow(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
        expect(invoke).toHaveBeenCalledWith('register_arrow', { namespace: 'ns@v1' });
    });
});

describe('useRemoveArrow', () => {
    it('calls invoke with remove_arrow command', async () => {
        const { result } = renderHook(() => useRemoveArrow(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'ns@v1' }));
        expect(invoke).toHaveBeenCalledWith('remove_arrow', { namespace: 'ns@v1' });
    });
});

describe('useFollowCollection', () => {
    it('calls invoke with follow_collection command', async () => {
        const { result } = renderHook(() => useFollowCollection(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
        expect(invoke).toHaveBeenCalledWith('follow_collection', { namespace: 'col/ns' });
    });
});

describe('useUnfollowCollection', () => {
    it('calls invoke with unfollow_collection command', async () => {
        const { result } = renderHook(() => useUnfollowCollection(), { wrapper: wrapper() });
        await act(() => result.current.mutateAsync({ namespace: 'col/ns' }));
        expect(invoke).toHaveBeenCalledWith('unfollow_collection', { namespace: 'col/ns' });
    });
});
