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
	namespace: NS,
	name: 'App',
	description: 'An app.',
	license: 'MIT',
	state: 'ready',
	tags: [],
	selector_kind: 'pin',
	resolved_ref: 'v1',
	installed_commit: '',
	outdated: false,
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
	manifest: {
		metadata: { url: '', maintainers: [], credits: [], media: {} },
		variables: [],
		netbridge: [],
		targets: {},
	},
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

	// This is the regression test for "install finished on the daemon but the
	// UI never shows it" on a first-time (Discovered) arrow: its route has no
	// ref yet -- ArrowDetailsScreen passes the bare namespace straight from
	// the URL splat (src/routes/arrow.$.tsx) -- but the live store, and every
	// runtime broadcast, key by the full `namespace@ref` form the daemon's own
	// detail response resolves to (`toArrowDetail`). If the live lookup uses
	// the raw, unresolved route param instead of that resolved form, it can
	// never find the entry the store is actually keyed under: every correct,
	// live update updates the STORE, but this hook's own `arrows.get(...)`
	// permanently misses it, and the page is stuck on its one-time fetch
	// forever -- not delayed, not eventually consistent, just wrong until a
	// full reload re-fetches from scratch.
	it('reflects a live runtime update even when called with the bare namespace a Discovered arrow only has before it resolves', async () => {
		const { result } = renderHook(() => useAssembledArrowDetail(BARE_NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'installing',
			active_run: { method: '_install', variables: {}, steps: [] },
			last_return: null,
		});

		await waitFor(() => expect(result.current.detail?.state).toBe('installing'));
	});

	it('re-reads the catalog when a run ends successfully, so the sidebar picks up the new resolved ref', async () => {
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'updating',
			active_run: { method: '_update', variables: {}, steps: [] },
			last_return: null,
		});
		await waitFor(() => expect(result.current.detail?.state).toBe('updating'));
		expect(refresh).not.toHaveBeenCalled();

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'ready',
			active_run: null,
			last_return: { method: '_update', outcome: 'success' },
		});
		await waitFor(() => expect(refresh).toHaveBeenCalledTimes(1));
	});

	it('does not re-read the catalog when a run ends in failure', async () => {
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());

		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'updating',
			active_run: { method: '_update', variables: {}, steps: [] },
			last_return: null,
		});
		useArrowStore.getState().applyRuntimeUpdate({
			namespace: NS,
			state: 'outdated',
			active_run: null,
			last_return: { method: '_update', outcome: 'failed' },
		});
		await waitFor(() => expect(result.current.detail?.state).toBe('outdated'));
		expect(refresh).not.toHaveBeenCalled();
	});

	it.each([
		['ready', 'outdated'],
		['outdated', 'ready'],
	] as const)(
		'refetches the detail when the runtime moves %s -> %s, so `available` is never stale',
		async (from, to) => {
			const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
			await waitFor(() => expect(result.current.detail).toBeDefined());
			useArrowStore
				.getState()
				.applyRuntimeUpdate({ namespace: NS, state: from, active_run: null, last_return: null });
			await waitFor(() => expect(result.current.detail?.state).toBe(from));
			mockApiFetch.mockClear();

			useArrowStore
				.getState()
				.applyRuntimeUpdate({ namespace: NS, state: to, active_run: null, last_return: null });

			await waitFor(() => expect(mockApiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(NS)}`));
		}
	);

	it('does not refetch for a transition that neither enters nor leaves outdated', async () => {
		const { result } = renderHook(() => useAssembledArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current.detail).toBeDefined());
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: NS, state: 'ready', active_run: null, last_return: null });
		await waitFor(() => expect(result.current.detail?.state).toBe('ready'));
		mockApiFetch.mockClear();

		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: NS, state: 'absent', active_run: null, last_return: null });
		await waitFor(() => expect(result.current.detail?.state).toBe('absent'));
		expect(mockApiFetch).not.toHaveBeenCalled();
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
		useArrowStore.getState().setCatalog([catalogRecord(), { ...catalogRecord(), namespace: other, name: 'Other' }]);
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
