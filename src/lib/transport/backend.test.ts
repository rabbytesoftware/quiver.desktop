import { afterEach, beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

// Partial, via `importOriginal`: `Channel` is imported by quiver-socket.ts and
// has to stay real. Only `invoke` -- the IPC boundary these tests stand in for
// -- is replaced.
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
	...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
	invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

import type { Backend, ConnectionsSnapshot, SocketLike } from './backend';
import { apiBase, backend, installBackend, realBackend, resetBackend, SOCKET_OPEN } from './backend';

const shell = window as unknown as { __QUIVER__?: { api?: string } };
const mockInvoke = invoke as MockedFunction<typeof invoke>;

function stubSocket(): SocketLike {
	return {
		readyState: SOCKET_OPEN,
		onopen: null,
		onmessage: null,
		onclose: null,
		onerror: null,
		send: vi.fn(),
		close: vi.fn(),
	};
}

function stubBackend(over: Partial<Backend> = {}): Backend {
	return {
		fetch: vi.fn().mockResolvedValue(new Response('{}')),
		openSocket: vi.fn(stubSocket),
		getBuildTag: vi.fn().mockResolvedValue(null),
		resolveReleaseAsset: vi.fn().mockRejectedValue({ kind: 'offline', detail: 'stub' }),
		getConnections: vi.fn().mockResolvedValue({ connections: [], active_id: 'stub' }),
		onCoreStatus: vi.fn().mockResolvedValue(() => {}),
		onConnectionsChanged: vi.fn().mockResolvedValue(() => {}),
		...over,
	};
}

beforeEach(() => {
	vi.restoreAllMocks();
	shell.__QUIVER__ = { api: 'quiver://localhost' };
});

afterEach(() => {
	resetBackend();
});

describe('the installed backend', () => {
	it('is the real one until something installs another', () => {
		expect(backend()).toBe(realBackend);
	});

	it('is whatever was installed last', () => {
		const first = stubBackend();
		const second = stubBackend();
		installBackend(first);
		installBackend(second);
		expect(backend()).toBe(second);
	});

	it('is visible to a consumer that resolved it before the install', async () => {
		const swapped = stubBackend();
		const resolveLater = () => backend();
		installBackend(swapped);
		expect(resolveLater()).toBe(swapped);
	});
});

describe('realBackend.fetch', () => {
	it('dials the shell-injected origin with the path appended', async () => {
		const fetchMock = vi.fn().mockResolvedValue(new Response('{}'));
		vi.stubGlobal('fetch', fetchMock);
		shell.__QUIVER__ = { api: 'http://quiver.localhost' };

		await realBackend.fetch('/v0/arrow', { method: 'POST' });

		expect(fetchMock).toHaveBeenCalledWith('http://quiver.localhost/v0/arrow', { method: 'POST' });
	});

	it('throws SYNCHRONOUSLY when there is no origin, rather than rejecting', () => {
		delete shell.__QUIVER__;
		expect(() => realBackend.fetch('/v0/health')).toThrow(/__QUIVER__/);
	});
});

describe('realBackend.getBuildTag', () => {
	it('hands back the release tag a stamped binary was built from', async () => {
		mockInvoke.mockResolvedValue('stable-26.5.1');

		await expect(realBackend.getBuildTag()).resolves.toBe('stable-26.5.1');
		expect(mockInvoke).toHaveBeenCalledWith('get_build_tag');
	});

	it('hands back null for a build that was never cut from a tag', async () => {
		// Rust answers `Option<String>`, so absence arrives as JSON null --
		// the ordinary case for every dev and CI build, not an error.
		mockInvoke.mockResolvedValue(null);

		await expect(realBackend.getBuildTag()).resolves.toBeNull();
	});

	it('does not invent a version when the command is unavailable', async () => {
		// An older shell without this command rejects the invoke. The caller
		// (announceSelf) is what decides to fall back; this layer must not
		// quietly substitute `tauri.conf.json`'s productVersion, which is what
		// the removed `getAppVersion` did and why the announce 404'd.
		mockInvoke.mockRejectedValue(new Error('command get_build_tag not found'));

		await expect(realBackend.getBuildTag()).rejects.toThrow(/get_build_tag/);
	});
});

describe('realBackend.resolveReleaseAsset', () => {
	it('hands back the asset the native side resolved, untouched', async () => {
		const asset = {
			tag: 'stable-26.9',
			name: 'quiver-desktop_0.1.0_amd64.AppImage',
			url: 'https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-26.9/quiver-desktop_0.1.0_amd64.AppImage',
			checksum: 'a'.repeat(64),
		};
		mockInvoke.mockResolvedValue(asset);

		await expect(realBackend.resolveReleaseAsset()).resolves.toEqual(asset);
		expect(mockInvoke).toHaveBeenCalledWith('resolve_release_asset');
	});

	// The typed failure is the whole point of the command returning a
	// `Result`: the caller branches on `kind` to pick a sentence, so this
	// layer must pass it through rather than flattening it into an Error.
	it('passes the native side’s typed failure through as it arrived', async () => {
		mockInvoke.mockRejectedValue({ kind: 'rate_limited', detail: 'GitHub answered 403' });

		await expect(realBackend.resolveReleaseAsset()).rejects.toEqual({
			kind: 'rate_limited',
			detail: 'GitHub answered 403',
		});
	});
});

describe('apiBase', () => {
	it('rejects an empty origin as firmly as a missing one', () => {
		shell.__QUIVER__ = { api: '' };
		expect(() => apiBase()).toThrow(/__QUIVER__/);
	});
});

describe('the interface', () => {
	it('numbers OPEN the way the WebSocket standard does', () => {
		expect(SOCKET_OPEN).toBe(1);
		expect(WebSocket.OPEN).toBe(SOCKET_OPEN);
	});

	it('lets a stand-in satisfy every method without touching Tauri', async () => {
		const snapshot: ConnectionsSnapshot = {
			connections: [{ id: 'stub', name: 'Stub', kind: 'local', api_version: 'v0' }],
			active_id: 'stub',
		};
		installBackend(stubBackend({ getConnections: vi.fn().mockResolvedValue(snapshot) }));

		await expect(backend().getConnections()).resolves.toEqual(snapshot);
		await expect(backend().getBuildTag()).resolves.toBeNull();
		await expect(backend().onCoreStatus(() => {})).resolves.toBeTypeOf('function');
		await expect(backend().onConnectionsChanged(() => {})).resolves.toBeTypeOf('function');
		expect(backend().openSocket('/v0/arrow').readyState).toBe(SOCKET_OPEN);
	});
});
