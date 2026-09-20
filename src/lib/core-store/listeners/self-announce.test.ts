import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

vi.mock('@/lib/transport/api', () => ({
	apiFetch: vi.fn(),
}));

import { apiFetch } from '@/lib/transport/api';

import { announceSelf } from './self-announce';

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;

/** The encoded form of `github.com/rabbytesoftware/quiver.desktop`, with no `@ref`. */
const SELF_PATH = '/v0/arrow/github.com%2Frabbytesoftware%2Fquiver.desktop';

beforeEach(() => {
	vi.clearAllMocks();
});

describe('announceSelf', () => {
	it("POSTs the ordinary add-arrow endpoint at quiver.desktop's own namespace", async () => {
		mockApiFetch.mockResolvedValue(undefined);

		await announceSelf();

		expect(apiFetch).toHaveBeenCalledTimes(1);
		expect(apiFetch).toHaveBeenCalledWith(SELF_PATH, { method: 'POST' });
	});

	it('announces no ref at all, so core resolves one from the real remote', async () => {
		mockApiFetch.mockResolvedValue(undefined);

		await announceSelf();

		// The regression this guards is specific: announcing `@0.1.0` (Tauri's
		// productVersion) demanded a git ref named `0.1.0`, which nothing in
		// this repo's release process ever creates, so the call 404'd forever.
		// A refless namespace is core's own `resolveRefless` path -- latest
		// stable release, else the default branch -- so the `%40` separator
		// must not reappear here.
		const [path] = mockApiFetch.mock.calls[0]!;
		expect(path).not.toContain('%40');
		expect(path).not.toContain('@');
	});

	it('logs and swallows a failure from the POST itself, rather than throwing', async () => {
		mockApiFetch.mockRejectedValue(new Error('502 bad gateway'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		logged.mockRestore();
	});
});
