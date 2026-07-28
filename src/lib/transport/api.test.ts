import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiFetch, isNotFoundError } from './api';

const NO_SLEEP = { attempts: 8, baseDelayMs: 0, maxDelayMs: 0, sleep: () => Promise.resolve() };

function jsonResponse(body: unknown, status = 200, headers: Record<string, string> = {}) {
	return new Response(JSON.stringify(body), { status, headers });
}

describe('apiFetch', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('unwraps a successful envelope', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ success: true, error: null, data: [1, 2] })));
		await expect(apiFetch<number[]>('/v0/arrow')).resolves.toEqual([1, 2]);
	});

	it('throws the envelope error message', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse({ success: false, error: 'nope' }, 200)));
		await expect(apiFetch('/v0/arrow')).rejects.toThrow('nope');
	});

	// A marked 502 means the request never reached the daemon: the socket was
	// refused. Retrying is the whole point — this is the app-launch window.
	it('retries an idempotent read on a proxy-marked 502', async () => {
		const fetchMock = vi
			.fn()
			.mockResolvedValueOnce(new Response('down', { status: 502, headers: { 'x-quiver-proxy': 'error' } }))
			.mockResolvedValueOnce(jsonResponse({ success: true, error: null, data: 'ok' }));
		vi.stubGlobal('fetch', fetchMock);
		await expect(apiFetch<string>('/v0/health', undefined, NO_SLEEP)).resolves.toBe('ok');
		expect(fetchMock).toHaveBeenCalledTimes(2);
	});

	// The default RetryConfig (no `sleep` override) is what every real call
	// site uses. Every other test injects NO_SLEEP's synchronous stub, which
	// would leave the real `setTimeout`-based backoff never exercised.
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

	// An UNMARKED 502 came from the daemon itself. Retrying it would replay
	// something that will never change, and hide a real server fault.
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

	it('treats 202 and 204 as success with no payload', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
		await expect(apiFetch('/v0/arrow/x', { method: 'DELETE' })).resolves.toBeUndefined();
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
