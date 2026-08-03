/** Mirrors `connection::proxy::PROXY_ERROR_HEADER`. */
export const PROXY_ERROR_HEADER = 'x-quiver-proxy';

const JSON_HEADERS = { 'content-type': 'application/json' };

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

export function accepted(): Response {
	return new Response(null, { status: 202 });
}

/** Unenveloped, because `/v0/health` is. `apiFetch` never touches it. */
export function bareHealth(): Response {
	return new Response(JSON.stringify({ status: 'ok' }), { status: 200, headers: JSON_HEADERS });
}

/**
 * A PROXY-shaped failure: nothing reached a daemon at all. The marker header is
 * what `apiFetch.isProxyFailure` keys its retry ladder on.
 */
export function unreachable(): Response {
	return new Response('mock: daemon unreachable', {
		status: 502,
		headers: { [PROXY_ERROR_HEADER]: 'error' },
	});
}
