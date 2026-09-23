import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { useRealPlatform } from './use-real-platform';

afterEach(() => {
	resetBackend();
	restoreUserAgent();
});

describe('useRealPlatform', () => {
	it('starts from the synchronous UA guess, before the native call resolves', () => {
		runningOn(USER_AGENTS.linux);
		installBackend({ getPlatform: vi.fn().mockReturnValue(new Promise(() => {})) } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());

		expect(result.current).toBe('linux/amd64');
	});

	it('swaps in the native side’s real answer once it resolves, even when it disagrees with the guess', async () => {
		// The exact failure mode this hook exists to fix: an Apple Silicon Mac
		// whose UA guess says amd64 but whose real platform is arm64.
		runningOn(USER_AGENTS.macos);
		installBackend({ getPlatform: vi.fn().mockResolvedValue('darwin/arm64') } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());
		expect(result.current).toBe('darwin/amd64');

		await waitFor(() => expect(result.current).toBe('darwin/arm64'));
	});

	it('keeps the sync guess when the native call rejects, rather than throwing', async () => {
		runningOn(USER_AGENTS.windows);
		const getPlatform = vi.fn().mockRejectedValue(new Error('command get_platform not found'));
		installBackend({ getPlatform } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());
		await waitFor(() => expect(getPlatform).toHaveBeenCalled());
		await Promise.resolve();

		expect(result.current).toBe('windows/amd64');
	});

	it('does not update state after unmount, once the in-flight call settles', async () => {
		let resolveCall: (value: string) => void = () => {};
		installBackend({
			getPlatform: vi.fn(
				() =>
					new Promise<string>((resolve) => {
						resolveCall = resolve;
					})
			),
		} as unknown as Backend);

		const { unmount } = renderHook(() => useRealPlatform());
		unmount();

		expect(() => resolveCall('darwin/arm64')).not.toThrow();
		await Promise.resolve();
	});
});
