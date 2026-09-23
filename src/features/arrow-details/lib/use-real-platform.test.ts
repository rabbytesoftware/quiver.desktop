import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { resolveRealPlatform, useRealPlatform } from './use-real-platform';

afterEach(() => {
	resetBackend();
	restoreUserAgent();
});

describe('useRealPlatform', () => {
	it('starts from the synchronous UA guess, unresolved, before the native call resolves', () => {
		runningOn(USER_AGENTS.linux);
		installBackend({ getPlatform: vi.fn().mockReturnValue(new Promise(() => {})) } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());

		expect(result.current).toEqual({ platform: 'linux/amd64', resolved: false });
	});

	it('swaps in the native side’s real answer once it resolves, even when it disagrees with the guess, and marks it resolved', async () => {
		// The exact failure mode this hook exists to fix: an Apple Silicon Mac
		// whose UA guess says amd64 but whose real platform is arm64.
		runningOn(USER_AGENTS.macos);
		installBackend({ getPlatform: vi.fn().mockResolvedValue('darwin/arm64') } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());
		expect(result.current).toEqual({ platform: 'darwin/amd64', resolved: false });

		await waitFor(() => expect(result.current).toEqual({ platform: 'darwin/arm64', resolved: true }));
	});

	it('keeps the sync guess, unresolved, when the native call rejects, rather than throwing', async () => {
		runningOn(USER_AGENTS.windows);
		const getPlatform = vi.fn().mockRejectedValue(new Error('command get_platform not found'));
		installBackend({ getPlatform } as unknown as Backend);

		const { result } = renderHook(() => useRealPlatform());
		await waitFor(() => expect(getPlatform).toHaveBeenCalled());
		await Promise.resolve();

		expect(result.current).toEqual({ platform: 'windows/amd64', resolved: false });
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

describe('resolveRealPlatform', () => {
	it('returns the native side’s real answer', async () => {
		installBackend({ getPlatform: vi.fn().mockResolvedValue('linux/arm64') } as unknown as Backend);

		await expect(resolveRealPlatform()).resolves.toBe('linux/arm64');
	});

	it('falls back to the UA guess when the native call rejects', async () => {
		runningOn(USER_AGENTS.macos);
		installBackend({ getPlatform: vi.fn().mockRejectedValue(new Error('unavailable')) } as unknown as Backend);

		await expect(resolveRealPlatform()).resolves.toBe('darwin/amd64');
	});
});
