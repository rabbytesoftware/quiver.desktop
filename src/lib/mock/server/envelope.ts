const PROXY_ERROR_HEADER = 'x-quiver-proxy';

const JSON_HEADERS = { 'content-type': 'application/json' };

export function ok(data: unknown, status = 200): Response {
	return new Response(JSON.stringify({ success: true, error: null, data }), {
		status,
		headers: JSON_HEADERS,
	});
}

export function fail(message: string, status = 400): Response {
	return new Response(JSON.stringify({ success: false, error: message, data: null }), {
		status,
		headers: JSON_HEADERS,
	});
}

export function accepted(): Response {
	return new Response(null, { status: 202 });
}

export function bareHealth(): Response {
	return new Response(JSON.stringify({ status: 'ok' }), { status: 200, headers: JSON_HEADERS });
}

export function unreachable(): Response {
	return new Response('mock: daemon unreachable', {
		status: 502,
		headers: { [PROXY_ERROR_HEADER]: 'error' },
	});
}
