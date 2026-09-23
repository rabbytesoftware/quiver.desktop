import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useArrowStore } from '@/lib/core-store/store/arrows';
import { apiFetch } from '@/lib/transport/api';

import { useAssembledArrowDetail } from './use-assembled-arrow-detail';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});

const mockApiFetch = vi.mocked(apiFetch);

const NS = 'github.com/user/app@v1';
const BARE_NS = 'github.com/user/app';

const DETAIL = {
	namespace: BARE_NS,
	name: 'App',
	version: 'v1',
	description: 'An app.',
	license: 'MIT',
	state: 'ready',
	tags: [],
	installed_ref: 'v1',
	installed_at: '2026-05-09T21:26:59Z',
	user_installed: true,
};

const MANIFEST = {
	namespace: BARE_NS,
	name: 'App',
	description: 'An app.',
	tags: [],
	variables: [],
	targets: {},
	manifest: { url: '', maintainers: [], credits: [], media: {}, netbridge: [] },
};

function catalogRecord() {
	return {
		connectionId: 'conn',
		namespace: NS,
		name: 'App',
		description: 'An app.',
		tags: [],
		icon: null,
		banner: null,
		version: 'v1',
	};
}

let client: QueryClient;

function wrapper({ children }: { children: ReactNode }) {
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockImplementation((path: string) => {
		if (path.endsWith('/manifest')) return Promise.resolve(MANIFEST);
		if (path.endsWith('/channels')) return Promise.resolve([]);
		if (path.endsWith('/readme')) return Promise.resolve(null);
		if (path.endsWith('/dependencies')) return Promise.resolve([]);
		if (path.endsWith('/dependents')) return Promise.resolve([]);
		return Promise.resolve(DETAIL);
	});
	client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	useArrowStore.getState().reset();
	useArrowStore.getState().setCatalog([catalogRecord()]);
});

describe('useAssembledArrowDetail', () => {
	it('refetches the detail query the moment a live run transitions from active to ended', async () => {
		const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());

		mockApiFetch.mockClear();

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'installing',
			active_run: { method: 'install', variables: {}, steps: [] },
			last_return: null,
		});
		await waitFor(() => expect(result.current.detail?.state).toBe('installing'));
		expect(mockApiFetch).not.toHaveBeenCalled();

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'ready',
			active_run: null,
			last_return: { method: 'install', outcome: 'failed' },
		});

		await waitFor(() => expect(mockApiFetch).toHaveBeenCalled());
	});

	it('does not refetch on mount when there is no active run to begin with', async () => {
		const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());

		mockApiFetch.mockClear();
		await new Promise((resolve) => setTimeout(resolve, 10));

		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('does not refetch on a genuinely different namespace after a prior one had an active run', async () => {
		const other = 'github.com/user/other@v1';
		useArrowStore.getState().setCatalog([
			catalogRecord(),
			{ ...catalogRecord(), namespace: other, name: 'Other' },
		]);
		mockApiFetch.mockImplementation((path: string) => {
			if (path.endsWith('/manifest')) return Promise.resolve({ ...MANIFEST, name: 'Other' });
			if (path.endsWith('/channels')) return Promise.resolve([]);
			if (path.endsWith('/readme')) return Promise.resolve(null);
			if (path.endsWith('/dependencies')) return Promise.resolve([]);
			if (path.endsWith('/dependents')) return Promise.resolve([]);
			return Promise.resolve({ ...DETAIL, name: 'Other' });
		});

		const { result, rerender } = renderHook(({ ns }) => useAssembledArrowDetail(ns), {
			wrapper,
			initialProps: { ns: NS },
		});
		await waitFor(() => expect(result.current.detail).toBeDefined());

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'installing',
			active_run: { method: 'install', variables: {}, steps: [] },
			last_return: null,
		});
		await waitFor(() => expect(result.current.detail?.state).toBe('installing'));

		rerender({ ns: other });
		await waitFor(() => expect(result.current.detail?.name).toBe('Other'));

		mockApiFetch.mockClear();
		await new Promise((resolve) => setTimeout(resolve, 10));

		expect(mockApiFetch).not.toHaveBeenCalled();
	});
});
