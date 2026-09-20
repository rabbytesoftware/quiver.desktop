import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

vi.mock('@/lib/transport/api', () => ({
	apiFetch: vi.fn(),
}));

vi.mock('@/lib/transport/backend', () => ({
	backend: vi.fn(),
}));

import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

import { announceSelf } from './self-announce';

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
const mockBackend = backend as MockedFunction<typeof backend>;

function stubBackend(getAppVersion: () => Promise<string>): void {
	mockBackend.mockReturnValue({
		fetch: vi.fn(),
		openSocket: vi.fn(),
		getConnections: vi.fn(),
		getAppVersion,
		onCoreStatus: vi.fn(),
		onCoreUpdateStatus: vi.fn(),
		onConnectionsChanged: vi.fn(),
	});
}

beforeEach(() => {
	vi.clearAllMocks();
});

describe('announceSelf', () => {
	it('POSTs the ordinary add-arrow endpoint at quiver.desktop\'s own namespace and version', async () => {
		stubBackend(() => Promise.resolve('0.1.0'));
		mockApiFetch.mockResolvedValue(undefined);

		await announceSelf();

		expect(apiFetch).toHaveBeenCalledTimes(1);
		expect(apiFetch).toHaveBeenCalledWith(
			'/v0/arrow/github.com%2Frabbytesoftware%2Fquiver.desktop%400.1.0',
			{ method: 'POST' }
		);
	});

	it('reads the version from Backend rather than hardcoding one', async () => {
		stubBackend(() => Promise.resolve('9.9.9-test'));
		mockApiFetch.mockResolvedValue(undefined);

		await announceSelf();

		expect(apiFetch).toHaveBeenCalledWith(
			'/v0/arrow/github.com%2Frabbytesoftware%2Fquiver.desktop%409.9.9-test',
			{ method: 'POST' }
		);
	});

	it('logs and swallows a failure reading the app version, without POSTing anything', async () => {
		stubBackend(() => Promise.reject(new Error('no tauri internals')));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		expect(apiFetch).not.toHaveBeenCalled();
		logged.mockRestore();
	});

	it('logs and swallows a failure from the POST itself, rather than throwing', async () => {
		stubBackend(() => Promise.resolve('0.1.0'));
		mockApiFetch.mockRejectedValue(new Error('502 bad gateway'));
		const logged = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(logged).toHaveBeenCalled();
		logged.mockRestore();
	});
});
