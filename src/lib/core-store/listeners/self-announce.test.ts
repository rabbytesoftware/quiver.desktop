import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

vi.mock('@/lib/transport/api', () => ({
	apiFetch: vi.fn(),
}));

vi.mock('@/lib/transport/backend', () => ({
	backend: vi.fn(),
}));

import { apiFetch } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { backend } from '@/lib/transport/backend';

import { announceSelf } from './self-announce';

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
const mockBackend = backend as MockedFunction<typeof backend>;

/** The encoded form of `github.com/rabbytesoftware/quiver.desktop`, with no `@ref`. */
const SELF_PATH = '/v0/arrow/github.com%2Frabbytesoftware%2Fquiver.desktop';

/** Stands in for a binary that was (or was not) built from a release tag. */
function builtFrom(tag: string | null): MockedFunction<Backend['getBuildTag']> {
	const getBuildTag = vi.fn().mockResolvedValue(tag) as MockedFunction<Backend['getBuildTag']>;
	mockBackend.mockReturnValue({ getBuildTag } as unknown as Backend);
	return getBuildTag;
}

function buildTagUnavailable(err: Error): void {
	mockBackend.mockReturnValue({
		getBuildTag: vi.fn().mockRejectedValue(err),
	} as unknown as Backend);
}

function announcedPath(): string {
	const call = mockApiFetch.mock.calls[0];
	if (!call) throw new Error('nothing was announced');
	return call[0];
}

beforeEach(() => {
	vi.clearAllMocks();
	mockApiFetch.mockResolvedValue(undefined);
	builtFrom(null);
});

describe('announceSelf, from a build cut from a real release tag', () => {
	it('announces that exact tag, so core resolves the ref this user is actually running', async () => {
		builtFrom('stable-26.5.1');

		await announceSelf();

		expect(apiFetch).toHaveBeenCalledTimes(1);
		expect(apiFetch).toHaveBeenCalledWith(`${SELF_PATH}%40stable-26.5.1`, { method: 'POST' });
	});

	it('carries the ref as one encoded namespace, `@` and all', async () => {
		builtFrom('stable-26.5');

		await announceSelf();

		// `%40` is the whole difference from the refless path below: core takes
		// an explicit ref as written (resolvers/http.go), so this row tracks
		// the tag this binary was built from rather than whatever the latest
		// published release happens to be.
		expect(announcedPath()).toBe(`${SELF_PATH}%40stable-26.5`);
	});
});

describe('announceSelf, from a build with no release tag', () => {
	it("POSTs the ordinary add-arrow endpoint at quiver.desktop's own namespace", async () => {
		await announceSelf();

		expect(apiFetch).toHaveBeenCalledTimes(1);
		expect(apiFetch).toHaveBeenCalledWith(SELF_PATH, { method: 'POST' });
	});

	it('announces no ref at all, so core resolves one from the real remote', async () => {
		await announceSelf();

		// The regression this guards is specific: announcing `@0.1.0` (Tauri's
		// productVersion) demanded a git ref named `0.1.0`, which nothing in
		// this repo's release process ever creates, so the call 404'd forever.
		// An UNSTAMPED build has no ref it can honestly claim either, so it
		// falls back to core's own `resolveRefless` path -- latest stable
		// release, else the default branch -- and the `%40` separator must not
		// reappear here.
		expect(announcedPath()).not.toContain('%40');
		expect(announcedPath()).not.toContain('@');
	});

	it('treats an empty tag as no tag rather than announcing a bare `@`', async () => {
		builtFrom('');

		await announceSelf();

		expect(apiFetch).toHaveBeenCalledWith(SELF_PATH, { method: 'POST' });
	});

	it('asks the backend once per announce, and announces once', async () => {
		const getBuildTag = builtFrom(null);

		await announceSelf();

		expect(getBuildTag).toHaveBeenCalledTimes(1);
		expect(apiFetch).toHaveBeenCalledTimes(1);
	});
});

describe('announceSelf, when something fails', () => {
	it('still announces -- refless -- when the build tag cannot be read at all', async () => {
		buildTagUnavailable(new Error('invoke unavailable'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		// The announce is the point; the tag only sharpens it. A backend that
		// cannot answer (an old shell, a webview with no IPC) must cost this
		// call its precision, never its existence.
		expect(apiFetch).toHaveBeenCalledWith(SELF_PATH, { method: 'POST' });
		expect(logged).toHaveBeenCalled();
		logged.mockRestore();
	});

	it('logs and swallows a failure from the POST itself, rather than throwing', async () => {
		mockApiFetch.mockRejectedValue(new Error('502 bad gateway'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		logged.mockRestore();
	});

	it('swallows a POST failure on the tagged path too', async () => {
		builtFrom('stable-26.5.1');
		mockApiFetch.mockRejectedValue(new Error('404 not found'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		logged.mockRestore();
	});
});
