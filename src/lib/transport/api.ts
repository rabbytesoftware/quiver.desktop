// Everything true of a quiver.core response regardless of which backend
// produced it: the `{success,error,data}` envelope, the retry ladder, and the
// proxy-vs-daemon distinction below.

import { backend } from './backend';

export { apiBase } from './backend';

/** Marks a response the Rust proxy generated rather than relayed. Mirrors
 *  connection::proxy::PROXY_ERROR_HEADER. */
const PROXY_ERROR_HEADER = 'x-quiver-proxy';

interface ApiEnvelope<T> {
	success: boolean;
	error: string | null;
	data?: T;
}

/** Carries the HTTP status so callers can make status-specific decisions —
 *  a 404 is terminal and meaningful, a 500 is a real server fault. */
export class ApiError extends Error {
	readonly status: number;

	constructor(message: string, status: number) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

export function isNotFoundError(err: unknown): boolean {
	return err instanceof ApiError && err.status === 404;
}

export interface RetryConfig {
	/** Total attempts including the first (1 = no retry). */
	attempts: number;
	baseDelayMs: number;
	maxDelayMs: number;
	sleep?: (ms: number) => Promise<void>;
}

// ~5s of bounded backoff, comfortably covering a daemon socket-bind window
// without masking a genuinely-down daemon.
const DEFAULT_RETRY: RetryConfig = { attempts: 8, baseDelayMs: 100, maxDelayMs: 1000 };

const defaultSleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

/** Only idempotent reads auto-retry. A mutation replayed at-least-once would
 *  double-apply, and quiver.core has no idempotency keys. */
function isIdempotentRead(init?: RequestInit): boolean {
	const method = init?.method?.toUpperCase();
	return method === undefined || method === 'GET';
}

/**
 * Whether this failure is the PROXY's rather than the daemon's.
 *
 * Under a URI-scheme proxy a refused socket never surfaces as a `fetch()`
 * rejection — it comes back as a well-formed 502 our own Rust handler built.
 * So retry cannot key on rejection the way a plain-origin client would; it
 * keys on the marker header instead. Without this distinction the frontend
 * would either never retry a cold start, or would replay real daemon 5xx.
 */
function isProxyFailure(res: Response): boolean {
	return (res.status === 502 || res.status === 504) && res.headers.get(PROXY_ERROR_HEADER) === 'error';
}

/**
 * Is the daemon answering RIGHT NOW?
 *
 * Deliberately NOT `apiFetch`, on two counts. `/v0/health` answers with a bare
 * `{"status":"ok"}` rather than the `{success,error,data}` envelope, so
 * `apiFetch` would reject a perfectly healthy daemon; and this is a snapshot,
 * not a wait — the caller wants to know whether the core is ALREADY up, so the
 * ~5s of retry backoff `apiFetch` applies to a marked 502 would be exactly the
 * wrong answer. Judged only on the HTTP status, which is the same question
 * `sidecar::wait_for_ready` asks on the Rust side.
 */
export async function coreIsReachable(): Promise<boolean> {
	// Issued OUTSIDE the catch: a missing API origin is a broken shell, not a
	// daemon that is down, and swallowing it into `false` would file it under
	// "backend unavailable". This is why `Backend.fetch` throws synchronously.
	const pending = backend().fetch('/v0/health');
	try {
		return (await pending).ok;
	} catch {
		// A `quiver://` request is answered by our own Rust proxy, which always
		// responds — but the webview can still reject the fetch outright (no
		// scheme handler registered, teardown mid-flight). Unreachable either way.
		return false;
	}
}

export async function apiFetch<T>(path: string, init?: RequestInit, retry: RetryConfig = DEFAULT_RETRY): Promise<T> {
	const maxAttempts = isIdempotentRead(init) ? Math.max(1, retry.attempts) : 1;
	const sleep = retry.sleep ?? defaultSleep;

	for (let attempt = 1; ; attempt++) {
		const res = await backend().fetch(path, init);

		if (isProxyFailure(res) && attempt < maxAttempts) {
			await sleep(Math.min(retry.maxDelayMs, retry.baseDelayMs * 2 ** (attempt - 1)));
			continue;
		}

		if (isProxyFailure(res)) {
			throw new ApiError(await res.text(), res.status);
		}

		const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;

		// A 202/204 or an empty body is success with no payload — the envelope
		// check below would wrongly throw on it.
		if (res.ok && (res.status === 204 || res.status === 202 || body === null)) {
			return undefined as T;
		}

		if (!res.ok || !body?.success) {
			throw new ApiError(body?.error ?? `${res.status} ${res.statusText}`, res.status);
		}

		return body.data as T;
	}
}
