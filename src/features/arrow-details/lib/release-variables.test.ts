import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ResolvedReleaseAsset } from '@/domain/release';
import type { Backend } from '@/lib/transport/backend';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import {
	isSelfArrow,
	releaseErrorMessageKey,
	releaseVariables,
	RELEASE_ASSET_URL,
	RELEASE_CHECKSUM,
	RELEASE_VARIABLE_NAMES,
} from './release-variables';

const SUM = 'a'.repeat(64);

const ASSET: ResolvedReleaseAsset = {
	tag: 'stable-26.9',
	name: 'quiver-desktop_0.1.0_amd64.AppImage',
	url: 'https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-26.9/quiver-desktop_0.1.0_amd64.AppImage',
	checksum: SUM,
};

function backendResolving(asset: ResolvedReleaseAsset) {
	const resolveReleaseAsset = vi.fn().mockResolvedValue(asset);
	installBackend({ resolveReleaseAsset } as unknown as Backend);
	return resolveReleaseAsset;
}

function backendFailing(err: unknown) {
	const resolveReleaseAsset = vi.fn().mockRejectedValue(err);
	installBackend({ resolveReleaseAsset } as unknown as Backend);
	return resolveReleaseAsset;
}

beforeEach(() => {
	vi.restoreAllMocks();
});

afterEach(() => {
	resetBackend();
});

describe('isSelfArrow', () => {
	it('recognises Quiver’s own row at any ref, and refless', () => {
		expect(isSelfArrow('github.com/rabbytesoftware/quiver.desktop')).toBe(true);
		expect(isSelfArrow('github.com/rabbytesoftware/quiver.desktop@stable-26.9')).toBe(true);
		// The ref an update runs AGAINST is the one being left behind, so a
		// row sitting at an older tag has to be recognised too -- that is the
		// only row the Update button ever appears on.
		expect(isSelfArrow('github.com/rabbytesoftware/quiver.desktop@stable-1.0')).toBe(true);
	});

	it('does not claim quiver.core, or anything that merely starts the same', () => {
		expect(isSelfArrow('github.com/rabbytesoftware/quiver.core@stable-26.5')).toBe(false);
		expect(isSelfArrow('github.com/rabbyte/minecraft@v1.21.4')).toBe(false);
		expect(isSelfArrow('github.com/rabbytesoftware/quiver.desktop.fork')).toBe(false);
	});
});

describe('releaseVariables', () => {
	it('resolves the two variables ARROW.md declares without defaults', async () => {
		backendResolving(ASSET);

		await expect(releaseVariables()).resolves.toEqual({
			[RELEASE_ASSET_URL]: ASSET.url,
			[RELEASE_CHECKSUM]: SUM,
		});
	});

	it('names exactly the variables the manifest declares', () => {
		expect(RELEASE_VARIABLE_NAMES).toEqual(['QUIVER_RELEASE_ASSET_URL', 'QUIVER_RELEASE_CHECKSUM']);
	});

	// THE STALE-VALUE GUARD, from this side. quiver.core's assembler no
	// longer carries a previous execution's value forward for a variable
	// declared with no default, and this side independently never depends on
	// that: every call goes back to the releases API, so a second update can
	// never reuse the first one's URL.
	it('asks the native side again on every call rather than remembering an answer', async () => {
		const resolve = backendResolving(ASSET);

		await releaseVariables();
		await releaseVariables();

		expect(resolve).toHaveBeenCalledTimes(2);
	});

	it('follows the asset when a second call resolves a different release', async () => {
		const resolve = vi
			.fn()
			.mockResolvedValueOnce(ASSET)
			.mockResolvedValueOnce({
				...ASSET,
				tag: 'stable-27.0',
				url: 'https://x/new.AppImage',
				checksum: 'b'.repeat(64),
			});
		installBackend({ resolveReleaseAsset: resolve } as unknown as Backend);

		await expect(releaseVariables()).resolves.toMatchObject({ [RELEASE_ASSET_URL]: ASSET.url });
		await expect(releaseVariables()).resolves.toMatchObject({ [RELEASE_ASSET_URL]: 'https://x/new.AppImage' });
	});

	// The policy decision, asserted so it cannot be softened by accident.
	// quiver.core's fetch step refuses a `${...}` checksum that resolved to
	// empty (ErrChecksumUnresolved) rather than skipping verification, so
	// "proceed unverified" is not a state this lifecycle can be put into --
	// starting the execution anyway would kill the running app and then fail.
	it('refuses an asset nothing published a checksum for', async () => {
		backendResolving({ ...ASSET, checksum: null });

		await expect(releaseVariables()).rejects.toMatchObject({ kind: 'unverifiable' });
	});

	it('never sends an empty checksum in place of a missing one', async () => {
		backendResolving({ ...ASSET, checksum: null });

		await expect(releaseVariables()).rejects.toBeTruthy();
	});

	it('passes the native side’s typed failure straight through', async () => {
		backendFailing({ kind: 'rate_limited', detail: 'GitHub answered 403' });

		await expect(releaseVariables()).rejects.toEqual({
			kind: 'rate_limited',
			detail: 'GitHub answered 403',
		});
	});

	// A webview whose IPC is unavailable rejects with a plain Error, not with
	// the typed failure. That must still produce something the UI can render.
	it('reads an untyped rejection as being unable to reach GitHub', async () => {
		backendFailing(new Error('command resolve_release_asset not found'));

		await expect(releaseVariables()).rejects.toMatchObject({ kind: 'offline' });
	});
});

describe('releaseErrorMessageKey', () => {
	it('gives each failure its own sentence', () => {
		expect(releaseErrorMessageKey('offline')).toBe('arrow.release.offline');
		expect(releaseErrorMessageKey('rate_limited')).toBe('arrow.release.rateLimited');
		expect(releaseErrorMessageKey('no_release')).toBe('arrow.release.noRelease');
		expect(releaseErrorMessageKey('no_asset')).toBe('arrow.release.noAsset');
		expect(releaseErrorMessageKey('unsupported_platform')).toBe('arrow.release.unsupportedPlatform');
		expect(releaseErrorMessageKey('unverifiable')).toBe('arrow.release.unverifiable');
	});

	it('falls back rather than rendering a raw key for a kind it does not know', () => {
		expect(releaseErrorMessageKey('something_new')).toBe('arrow.release.unavailable');
		expect(releaseErrorMessageKey('')).toBe('arrow.release.unavailable');
	});
});
