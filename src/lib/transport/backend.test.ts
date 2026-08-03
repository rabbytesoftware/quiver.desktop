import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Backend, ConnectionsSnapshot, SocketLike } from './backend';
import { apiBase, backend, installBackend, realBackend, resetBackend, SOCKET_OPEN } from './backend';

const shell = window as unknown as { __QUIVER__?: { api?: string } };

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

	// Every consumer resolves `backend()` per call rather than capturing it, so
	// that a boot-time install is visible to modules imported before it ran —
	// which is all of them, since `main.tsx` installs after its own imports.
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

	// The contract `coreIsReachable` leans on: a backend with no origin is a
	// broken SHELL, and must be distinguishable from a daemon that is merely
	// down. A rejected promise would land in that function's catch and be
	// reported as "core unavailable" — the exact ambiguity `apiBase` refuses.
	it('throws SYNCHRONOUSLY when there is no origin, rather than rejecting', () => {
		delete shell.__QUIVER__;
		expect(() => realBackend.fetch('/v0/health')).toThrow(/__QUIVER__/);
	});
});

describe('apiBase', () => {
	it('rejects an empty origin as firmly as a missing one', () => {
		shell.__QUIVER__ = { api: '' };
		expect(() => apiBase()).toThrow(/__QUIVER__/);
	});
});

describe('the interface', () => {
	// SOCKET_OPEN is restated here rather than read off `WebSocket`, so this
	// pins it: `wsManager` compares `readyState` against it to decide whether a
	// socket is writable, and every implementation — real, mock, test fake —
	// has to agree on the number the standard fixed.
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
		await expect(backend().onCoreStatus(() => {})).resolves.toBeTypeOf('function');
		await expect(backend().onConnectionsChanged(() => {})).resolves.toBeTypeOf('function');
		expect(backend().openSocket('/v0/arrow').readyState).toBe(SOCKET_OPEN);
	});
});
