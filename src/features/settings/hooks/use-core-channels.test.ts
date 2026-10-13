import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { apiFetch } from '@/lib/transport/api';

import { useCoreChannels } from './use-core-channels';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));
const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

beforeEach(() => {
	mockApiFetch.mockReset();
});

describe('useCoreChannels', () => {
	it('starts loading and fetches quiver.core’s own channels endpoint', () => {
		mockApiFetch.mockReturnValue(new Promise(() => {}));
		const { result } = renderHook(() => useCoreChannels());

		expect(result.current).toEqual({ channels: [], loading: true, failed: false });
		expect(apiFetch).toHaveBeenCalledWith(`/v0/arrow/${encodeURIComponent(QUIVER_CORE_NAMESPACE)}/channels`);
	});

	it('resolves to the mapped channel list on success', async () => {
		mockApiFetch.mockResolvedValue({
			channels: [{ name: 'stable', kind: 'ordered', latest: 'v1', count: 1, members: ['v1'] }],
		});
		const { result } = renderHook(() => useCoreChannels());

		await waitFor(() => expect(result.current.loading).toBe(false));
		expect(result.current.failed).toBe(false);
		expect(result.current.channels).toEqual([
			{ name: 'stable', kind: 'ordered', latest: 'v1', count: 1, members: ['v1'] },
		]);
	});

	it('reports failed, with an empty channel list, when the fetch rejects -- most commonly a 404 before self-registration completes', async () => {
		mockApiFetch.mockRejectedValue(new Error('not found'));
		const { result } = renderHook(() => useCoreChannels());

		await waitFor(() => expect(result.current.loading).toBe(false));
		expect(result.current.failed).toBe(true);
		expect(result.current.channels).toEqual([]);
	});

	it('does not update state after the hook has unmounted, once the in-flight fetch settles', async () => {
		let resolveFetch: (value: { channels: never[] }) => void = () => {};
		mockApiFetch.mockReturnValue(
			new Promise((resolve) => {
				resolveFetch = resolve;
			})
		);
		const { unmount } = renderHook(() => useCoreChannels());

		unmount();
		// Resolving after unmount must not throw (React would warn/error on a
		// state update past unmount if the effect's cleanup guard were missing).
		expect(() => resolveFetch({ channels: [] })).not.toThrow();
		await Promise.resolve();
	});
});
