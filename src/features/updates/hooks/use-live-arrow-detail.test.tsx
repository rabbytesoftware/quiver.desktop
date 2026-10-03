import type { ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useArrowStore } from '@/lib/core-store';
import { apiFetch } from '@/lib/transport/api';

import { useLiveArrowDetail } from './use-live-arrow-detail';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn() };
});

const mockApiFetch = vi.mocked(apiFetch);

const NS = 'github.com/rabbytesoftware/quiver.core@stable';

const DETAIL = {
	namespace: NS,
	name: 'Quiver Core',
	description: '',
	state: 'ready',
	tags: [],
	selector_kind: 'channel',
	resolved_ref: 'stable-26.5',
	installed_commit: '',
	outdated: false,
	user_installed: true,
	pending_activation: { version: 'stable-26.6', staged_at: '2026-10-03T10:00:00Z' },
};

const MANIFEST = {
	namespace: 'github.com/rabbytesoftware/quiver.core',
	name: 'Quiver Core',
	description: '',
	tags: [],
	variables: [],
	targets: {},
	manifest: { metadata: {}, variables: [], netbridge: [], targets: {} },
};

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
	mockApiFetch
		.mockReset()
		.mockImplementation((path: string) => Promise.resolve(path.endsWith('/manifest') ? MANIFEST : DETAIL));
	useArrowStore.getState().reset();
	useArrowStore.getState().setCatalog([
		{
			connectionId: 'c',
			namespace: NS,
			name: 'Quiver Core',
			description: '',
			tags: [],
			icon: null,
			banner: null,
			version: 'stable-26.5',
		},
	]);
});

describe('useLiveArrowDetail', () => {
	it('is undefined and fetches nothing without a namespace', () => {
		const { result } = renderHook(() => useLiveArrowDetail(''), { wrapper });
		expect(result.current).toBeUndefined();
		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('returns the fetched detail, staging included', async () => {
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: NS, state: 'ready', active_run: null, last_return: null });
		const { result } = renderHook(() => useLiveArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current).toBeDefined());
		expect(result.current?.pending_activation?.version).toBe('stable-26.6');
		expect(result.current?.state).toBe('ready');
	});

	it('overlays the live state and run from the store, including a run that has ended', async () => {
		const { result } = renderHook(() => useLiveArrowDetail(NS), { wrapper });
		await waitFor(() => expect(result.current).toBeDefined());

		act(() => {
			useArrowStore.getState().applyRuntimeUpdate({
				namespace: NS,
				state: 'updating',
				active_run: { method: 'update', variables: {}, steps: [] },
				last_return: null,
			});
		});
		await waitFor(() => expect(result.current?.state).toBe('updating'));
		expect(result.current?.active_run?.method).toBe('update');

		act(() => {
			useArrowStore.getState().applyRuntimeUpdate({
				namespace: NS,
				state: 'ready',
				active_run: null,
				last_return: null,
			});
		});
		await waitFor(() => expect(result.current?.active_run).toBeNull());
	});
});
