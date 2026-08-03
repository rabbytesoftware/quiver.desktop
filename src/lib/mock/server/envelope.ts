// quiver.core's response shapes, reproduced exactly. `apiFetch` unwraps these
// for real against the mock — it is the same code path a live daemon drives —
// so getting the envelope wrong here surfaces as the app misbehaving rather
// than as a mock that is merely inaccurate.

/** Mirrors `connection::proxy::PROXY_ERROR_HEADER`. */
export const PROXY_ERROR_HEADER = 'x-quiver-proxy';

const JSON_HEADERS = { 'content-type': 'application/json' };

/** `{success: true, error: null, data}` — what every /v0 route but health returns. */
export function ok(data: unknown, status = 200): Response {
	return new Response(JSON.stringify({ success: true, error: null, data }), {
		status,
		headers: JSON_HEADERS,
	});
}

/** A DAEMON-shaped failure: the request reached quiver.core and it said no. */
export function fail(message: string, status = 400): Response {
	return new Response(JSON.stringify({ success: false, error: message, data: null }), {
		status,
		headers: JSON_HEADERS,
	});
}

/**
 * Success with no payload. Core answers 202 on accepted runtime verbs, and
 * `apiFetch` has a dedicated branch for it — a 202 must NOT be read as an
 * envelope, or every install would throw on a response that meant "started".
 */
export function accepted(): Response {
	return new Response(null, { status: 202 });
}

/**
 * Unenveloped, because `/v0/health` is. `coreIsReachable` judges it on the HTTP
 * status alone and `apiFetch` never touches it — a health route wrapped in the
 * envelope would make a perfectly healthy daemon look broken.
 */
export function bareHealth(): Response {
	return new Response(JSON.stringify({ status: 'ok' }), { status: 200, headers: JSON_HEADERS });
}

/**
 * A PROXY-shaped failure: nothing reached a daemon at all.
 *
 * Distinct from `fail` in the one way that matters — the marker header — which
 * is what `apiFetch.isProxyFailure` keys its retry ladder on. Under a
 * URI-scheme proxy a refused socket never surfaces as a rejected fetch; it
 * comes back as a well-formed 502 the Rust handler built, and this imitates
 * that so the retry path can be exercised without a daemon to kill.
 */
export function unreachable(): Response {
	return new Response('mock: daemon unreachable', {
		status: 502,
		headers: { [PROXY_ERROR_HEADER]: 'error' },
	});
}
