import type { FaultKey } from '../store';
import { applyChaos, type Rng } from './chaos';
import { fail } from './envelope';
import type { MockWorld } from '../world/types';

export interface MockRequest {
	method: string;
	path: string;
	query: URLSearchParams;
	/** Already percent-decoded. */
	params: Record<string, string>;
	body: unknown;
}

export type Handler = (req: MockRequest, world: MockWorld) => Response | Promise<Response>;

export interface Route {
	method: 'GET' | 'POST' | 'DELETE';
	/** `/v0/runtime/:ns/:verb`. */
	pattern: string;

	fault: FaultKey;
	handler: Handler;
}

/**
 * Segment-wise rather than a path-spanning regex. Namespaces contain `/`, `@`
 * and `.`, but every caller encodes them, so a namespace is always exactly one
 * segment on the wire.
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
					// A broken fixture and a failing daemon look identical from the UI,
					// so the log names the route.
					console.error(`mock: handler for ${method} ${route.pattern} threw`, err);
					return fail(`mock handler error on ${method} ${path}`, 500);
				}
			}

			// Loud: an unrouted path answered with an empty 200 renders as "you have
			// no arrows", a plausible screen that is a lie.
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
