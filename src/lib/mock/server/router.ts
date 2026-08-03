import type { FaultKey } from '../store';
import { applyChaos, type Rng } from './chaos';
import { fail } from './envelope';
import type { MockWorld } from '../world/types';

export interface MockRequest {
	method: string;
	path: string;
	query: URLSearchParams;
	/** Path captures, already percent-DECODED. */
	params: Record<string, string>;
	body: unknown;
}

export type Handler = (req: MockRequest, world: MockWorld) => Response | Promise<Response>;

export interface Route {
	method: 'GET' | 'POST' | 'DELETE';
	/** `/v0/runtime/:ns/:verb`. A `:name` segment captures one decoded segment. */
	pattern: string;
	/** Which Developer-tab fault slider can break this route. */
	fault: FaultKey;
	handler: Handler;
}

/**
 * Segment-wise, and that is enough — deliberately not a regex over the whole
 * path.
 *
 * Namespaces contain `/`, `@` and `.` (`github.com/rabbyte/minecraft@v1.21.4`),
 * which would make a path-spanning pattern ambiguous. But every caller sends
 * them through `encodeURIComponent`, so the slashes arrive as `%2F` and a
 * namespace is always exactly ONE segment on the wire. Splitting on `/` and
 * decoding each capture is therefore both simpler and more correct than any
 * pattern that tried to be clever about it.
 */
function matchPattern(pattern: string, path: string): Record<string, string> | null {
	const patternParts = pattern.split('/');
	const pathParts = path.split('/');
	if (patternParts.length !== pathParts.length) return null;

	const params: Record<string, string> = {};
	for (let i = 0; i < patternParts.length; i++) {
		const expected = patternParts[i];
		const actual = pathParts[i];
		if (expected.startsWith(':')) {
			params[expected.slice(1)] = decodeURIComponent(actual);
			continue;
		}
		if (expected !== actual) return null;
	}
	return params;
}

export interface Router {
	handle(path: string, init: RequestInit | undefined, world: MockWorld): Promise<Response>;
}

export function createRouter(routes: Route[], rng: Rng = Math.random): Router {
	return {
		async handle(pathWithQuery, init, world) {
			const method = (init?.method ?? 'GET').toUpperCase();
			const [path, search = ''] = pathWithQuery.split('?');
			const query = new URLSearchParams(search);

			for (const route of routes) {
				if (route.method !== method) continue;
				const params = matchPattern(route.pattern, path);
				if (!params) continue;

				const interrupted = await applyChaos(route.fault, rng);
				if (interrupted) return interrupted;

				try {
					return await route.handler({ method, path, query, params, body: parseBody(init) }, world);
				} catch (err) {
					// A fixture that throws is a bug in the mock, not in the app, and
					// the two are indistinguishable from the UI — both look like the
					// daemon failing. Naming the route in the log is the difference
					// between a five-minute fix and an afternoon.
					console.error(`mock: handler for ${method} ${route.pattern} threw`, err);
					return fail(`mock handler error on ${method} ${path}`, 500);
				}
			}

			// Loud on purpose. An unrouted path answered with an empty 200 would
			// render as "you have no arrows" — a plausible screen that is a lie,
			// and the single most expensive way for this mock to be wrong.
			return fail(`mock: no route for ${method} ${path}`, 404);
		},
	};
}

function parseBody(init: RequestInit | undefined): unknown {
	if (typeof init?.body !== 'string') return null;
	try {
		return JSON.parse(init.body);
	} catch {
		return init.body;
	}
}
