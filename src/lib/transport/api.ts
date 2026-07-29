// Every HTTP call the app makes, over the scheme handler the Rust proxy
// registers. The proxy resolves the ACTIVE connection per request, so nothing
// here knows or cares whether the daemon is local or remote.

/**
 * The origin every request is sent to — whatever the shell injected, and
 * nothing else.
 *
 * There is deliberately NO fallback literal. The reachable origin is not the
 * same on every platform: WebView2 cannot register a non-standard scheme, so
 * wry registers `http://quiver.localhost` and rewrites requests back before
 * handing them to the handler, while WKWebView and WebKitGTK serve
 * `quiver://localhost` as written. Any literal here is therefore silently
 * correct on two platforms and silently wrong on the third, and nothing can
 * see it: it is a string, `tsc` has no opinion on it, and a suite that runs on
 * macOS and Linux never dials the origin Windows needs. So the shell computes
 * it once per platform (`QUIVER_API_BASE`, src-tauri/src/lib.rs) and injects it
 * at document-start; this module takes what it is given.
 *
 * Absent config means one of two things, and neither has a correct origin to
 * guess at: the page is not inside the Tauri shell (a bare `vite` dev server,
 * which cannot reach the daemon by any URL), or the init script did not run.
 * Both are diagnosable the moment they say so, and mysterious the moment they
 * are papered over with a plausible-looking default — a wrong base fails as an
 * ordinary network error several layers away from its cause. So it throws.
 *
 * Resolved per call rather than at module load, so that merely IMPORTING this
 * module outside the shell stays harmless — the test suite does exactly that,
 * every time it mocks `apiFetch` — and the failure lands on the caller that
 * actually wanted the network.
 */
export function apiBase(): string {
	const base = (window as unknown as { __QUIVER__?: { api?: string } }).__QUIVER__?.api;
	if (base) return base;
	throw new Error(
		'window.__QUIVER__.api is not set, so there is no API origin to dial. The page is either not running ' +
			'inside the Quiver shell (which injects a per-platform origin at document-start) or that injection failed.'
	);
}

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
	// Resolved OUTSIDE the catch, on purpose. A missing API origin is a broken
	// shell, not a daemon that is down, and swallowing it into `false` would
	// file it under the app's ordinary "backend unavailable" state — the exact
	// unexplainable failure `apiBase` exists to refuse.
	const base = apiBase();
	try {
		return (await fetch(`${base}/v0/health`)).ok;
	} catch {
		// A `quiver://` request is answered by our own Rust proxy, which always
		// responds — but the webview can still reject the fetch outright (no
		// scheme handler registered, teardown mid-flight). Unreachable either way.
		return false;
	}
}

export async function apiFetch<T>(path: string, init?: RequestInit, retry: RetryConfig = DEFAULT_RETRY): Promise<T> {
	const base = apiBase();
	const maxAttempts = isIdempotentRead(init) ? Math.max(1, retry.attempts) : 1;
	const sleep = retry.sleep ?? defaultSleep;

	for (let attempt = 1; ; attempt++) {
		const res = await fetch(`${base}${path}`, init);

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
