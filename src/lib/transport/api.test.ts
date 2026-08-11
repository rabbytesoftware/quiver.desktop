import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiBase, apiFetch, coreIsReachable, isNotFoundError } from './api';

const NO_SLEEP = { attempts: 8, baseDelayMs: 0, maxDelayMs: 0, sleep: () => Promise.resolve() };

const SHELL_BASE = 'quiver://localhost';

const shell = window as unknown as { __QUIVER__?: { api?: string } };

function jsonResponse(body: unknown, status = 200, headers: Record<string, string> = {}) {
	return new Response(JSON.stringify(body), { status, headers });
}

beforeEach(() => {
	vi.restoreAllMocks();
	shell.__QUIVER__ = { api: SHELL_BASE };
});

describe('apiBase', () => {
	it('dials the origin the shell injected, whatever it is', async () => {
		shell.__QUIVER__ = { api: 'http://quiver.localhost' };
		const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ success: true, error: null, data: 'ok' }));
		vi.stubGlobal('fetch', fetchMock);
		await apiFetch('/v0/arrow');
		expect(fetchMock.mock.calls[0][0]).toBe('http://quiver.localhost/v0/arrow');
	});

	it('refuses to guess an origin when the shell injected none', async () => {
		delete shell.__QUIVER__;
		const fetchMock = vi.fn();
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch('/v0/arrow')).rejects.toThrow(/__QUIVER__/);
		expect(fetchMock).not.toHaveBeenCalled();
	});

	it('treats an empty injected origin as no origin', () => {
		shell.__QUIVER__ = { api: '' };
		expect(() => apiBase()).toThrow(/__QUIVER__/);
	});

	it('surfaces a missing origin from coreIsReachable instead of reporting the core down', async () => {
		delete shell.__QUIVER__;
		vi.stubGlobal('fetch', vi.fn());
		await expect(coreIsReachable()).rejects.toThrow(/__QUIVER__/);
	});
});

describe('apiFetch', () => {
	it('unwraps a successful envelope', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ success: true, error: null, data: [1, 2] })));
		await expect(apiFetch<number[]>('/v0/arrow')).resolves.toEqual([1, 2]);
	});

	it('throws the envelope error message', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ success: false, error: 'nope' }, 200)));
		await expect(apiFetch('/v0/arrow')).rejects.toThrow('nope');
	});

	it('retries an idempotent read on a proxy-marked 502', async () => {
		const fetchMock = vi
			.fn()
			.mockResolvedValueOnce(new Response('down', { status: 502, headers: { 'x-quiver-proxy': 'error' } }))
			.mockResolvedValueOnce(jsonResponse({ success: true, error: null, data: 'ok' }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch<string>('/v0/health', undefined, NO_SLEEP)).resolves.toBe('ok');
		expect(fetchMock).toHaveBeenCalledTimes(2);
	});

	it('recovers via the default retry config, with its real backoff timer', async () => {
		vi.useFakeTimers();
		const fetchMock = vi
			.fn()
			.mockResolvedValueOnce(new Response('down', { status: 502, headers: { 'x-quiver-proxy': 'error' } }))
			.mockResolvedValueOnce(jsonResponse({ success: true, error: null, data: 'ok' }));
		vi.stubGlobal('fetch', fetchMock);
		const result = apiFetch<string>('/v0/health');
		await vi.advanceTimersByTimeAsync(1000);
		await expect(result).resolves.toBe('ok');
		expect(fetchMock).toHaveBeenCalledTimes(2);
		vi.useRealTimers();
	});

	it('does not retry an unmarked 502', async () => {
		const fetchMock = vi.fn().mockResolvedValue(new Response('{}', { status: 502 }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch('/v0/health', undefined, NO_SLEEP)).rejects.toThrow(ApiError);
		expect(fetchMock).toHaveBeenCalledTimes(1);
	});

	it('never retries a mutation, even when marked', async () => {
		const fetchMock = vi
			.fn()
			.mockResolvedValue(new Response('down', { status: 502, headers: { 'x-quiver-proxy': 'error' } }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch('/v0/arrow/x', { method: 'POST' }, NO_SLEEP)).rejects.toThrow(ApiError);
		expect(fetchMock).toHaveBeenCalledTimes(1);
	});

	it('surfaces a 404 as a terminal ApiError', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ success: false, error: 'gone' }, 404)));
		const err = await apiFetch('/v0/arrow/x', undefined, NO_SLEEP).catch((e: unknown) => e);
		expect(isNotFoundError(err)).toBe(true);
	});

	it('treats 204 as success with no payload', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
		await expect(apiFetch('/v0/arrow/x', { method: 'DELETE' })).resolves.toBeUndefined();
	});

	it('treats 202 as success even with a non-envelope body', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ queued: true }, 202)));
		await expect(apiFetch('/v0/arrow/x', { method: 'POST' })).resolves.toBeUndefined();
	});

	it('gives up after the configured attempts', async () => {
		const fetchMock = vi
			.fn()
			.mockResolvedValue(new Response('down', { status: 504, headers: { 'x-quiver-proxy': 'error' } }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch('/v0/health', undefined, { ...NO_SLEEP, attempts: 3 })).rejects.toThrow(ApiError);
		expect(fetchMock).toHaveBeenCalledTimes(3);
	});
});

describe('coreIsReachable', () => {
	it('reports a healthy daemon as reachable despite its unenveloped body', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ status: 'ok' })));
		await expect(coreIsReachable()).resolves.toBe(true);
	});

	it('probes /v0/health exactly once, without retrying a proxy-marked 502', async () => {
		const fetchMock = vi
			.fn()
			.mockResolvedValue(new Response('down', { status: 502, headers: { 'x-quiver-proxy': 'error' } }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(coreIsReachable()).resolves.toBe(false);
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(fetchMock.mock.calls[0][0]).toContain('/v0/health');
	});

	it('reports unreachable when the fetch itself rejects', async () => {
		vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('scheme handler gone')));
		await expect(coreIsReachable()).resolves.toBe(false);
	});
});
