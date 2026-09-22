import { createElement, type ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import type { ArrowChannel, ArrowDetail } from '@/domain/arrow';
import { apiFetch } from '@/lib/transport/api';

import { useChannelSelection } from './use-channel-selection';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

const STABLE: ArrowChannel = { name: 'stable', kind: 'ordered', latest: 'v2', count: 2, members: ['v2', 'v1'] };
const BETA: ArrowChannel = { name: 'beta', kind: 'ordered', latest: 'v3-beta', count: 1, members: ['v3-beta'] };

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: 'github.com/rabbyte/x@v2',
		name: 'X',
		description: '',
		license: '',
		url: '',
		tags: [],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [],
		targets: [],
		state: 'ready',
		user_installed: true,
		installed_ref: 'v2',
		channel: 'stable',
		channels: [STABLE, BETA],
		active_run: null,
		last_return: null,
		readme: null,
		dependencies: [],
		dependents: [],
		...overrides,
	};
}

function wrapper() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
	return function Wrapper({ children }: { children: ReactNode }) {
		return createElement(QueryClientProvider, { client }, children);
	};
}

beforeEach(() => {
	mockApiFetch.mockReset();
	mockApiFetch.mockResolvedValue(undefined);
});

describe('useChannelSelection', () => {
	it('seeds from detail.channel, falling back to the first published channel when untracked', () => {
		const { result, rerender } = renderHook(({ d }) => useChannelSelection(d), {
			wrapper: wrapper(),
			initialProps: { d: detail({ channel: undefined }) },
		});
		expect(result.current.selectedChannel).toBe('stable');

		// A genuinely different arrow (new namespace) re-seeds from scratch --
		// note the reversed channel order proves this re-ran rather than just
		// happening to already be 'stable'.
		rerender({
			d: detail({ channel: undefined, channels: [BETA, STABLE], namespace: 'github.com/rabbyte/y@v1' }),
		});
		expect(result.current.selectedChannel).toBe('beta');
	});

	it('skips the core call for a target detail already reflects', () => {
		const { result } = renderHook(() => useChannelSelection(detail({ channel: 'stable', installed_ref: 'v2' })), {
			wrapper: wrapper(),
		});

		act(() => {
			result.current.selectVersion('stable', 'v2');
		});

		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('skips the core call entirely for a not-yet-installed arrow, but still updates local state', () => {
		const { result } = renderHook(() => useChannelSelection(detail({ user_installed: false })), {
			wrapper: wrapper(),
		});

		act(() => {
			result.current.selectChannel('beta');
		});

		expect(apiFetch).not.toHaveBeenCalled();
		expect(result.current.selectedChannel).toBe('beta');
	});

	// Regression: a genuine concurrent double-click (Channel then Version, or
	// vice versa) fired two overlapping PATCHes against the same starting
	// namespace live -- `useMutation`'s own `isPending` only updates on
	// React's next render, so it could not have caught this. `selectChannel`
	// and `selectVersion` are invoked synchronously back to back, inside one
	// `act`, before the first mutation's promise ever settles.
	it('drops a second switch fired synchronously before the first settles, sending only one PATCH', async () => {
		let resolveFirst: (value: undefined) => void = () => {};
		mockApiFetch.mockReturnValueOnce(
			new Promise((resolve) => {
				resolveFirst = resolve;
			})
		);

		const { result } = renderHook(() => useChannelSelection(detail()), { wrapper: wrapper() });

		act(() => {
			result.current.selectChannel('beta');
			result.current.selectVersion('stable', 'v1');
		});

		// react-query dispatches the mutation function on a microtask, not
		// synchronously within the call to `mutateAsync` -- but the `inFlight`
		// guard itself is set synchronously inside `switchOnCore`, before that
		// dispatch, which is the whole point: by the time either call could
		// reach here, the second one has already been turned away.
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(1));
		// The winning call (the channel pick, fired first) is what local state
		// reflects -- not a mix with the dropped version pick's own target.
		expect(result.current.selectedChannel).toBe('beta');
		expect(result.current.selectedVersion).toBe('v3-beta');

		resolveFirst(undefined);
	});

	it('also drops a channel pick that loses the race to an in-flight version pick', async () => {
		let resolveFirst: (value: undefined) => void = () => {};
		mockApiFetch.mockReturnValueOnce(
			new Promise((resolve) => {
				resolveFirst = resolve;
			})
		);

		const { result } = renderHook(() => useChannelSelection(detail()), { wrapper: wrapper() });

		act(() => {
			result.current.selectVersion('stable', 'v1');
			result.current.selectChannel('beta');
		});

		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(1));
		expect(result.current.selectedChannel).toBe('stable');
		expect(result.current.selectedVersion).toBe('v1');

		resolveFirst(undefined);
	});

	it('allows a new switch once the in-flight one has settled', async () => {
		// `detail` here is a static fixture (this test doesn't simulate the
		// server-refetch a real switch triggers), so both attempts target
		// `beta` -- targeting `stable` again would be silently skipped by the
		// separate "already matches detail" no-op guard instead of exercising
		// the in-flight one this test is actually about.
		const { result } = renderHook(() => useChannelSelection(detail()), { wrapper: wrapper() });

		act(() => {
			result.current.selectChannel('beta');
		});
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(1));

		act(() => {
			result.current.selectChannel('beta');
		});
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(2));
	});

	it('still clears the in-flight guard when the switch rejects, so a later attempt is not stuck blocked', async () => {
		mockApiFetch.mockRejectedValueOnce(new Error('offline'));
		const { result } = renderHook(() => useChannelSelection(detail()), { wrapper: wrapper() });

		act(() => {
			result.current.selectChannel('beta');
		});
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(1));

		mockApiFetch.mockResolvedValueOnce(undefined);
		act(() => {
			result.current.selectChannel('beta');
		});
		await waitFor(() => expect(apiFetch).toHaveBeenCalledTimes(2));
	});
});
