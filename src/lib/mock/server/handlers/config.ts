import { flattenConfig } from '@/lib/core-store/dtos/v0/config';

import { CONFIG_DEFAULTS, type MockConfigDoc, type MockWorld } from '../../world/types';
import { ok } from '../envelope';
import type { Route } from '../router';

const LEVELS = ['debug', 'trace', 'info', 'warn', 'warning', 'error', 'fatal', 'panic'];

// Seeds the panel's `corrected` notice from a test. Lives on the world (not
// module state) so `installMock()` alone is a complete reset — reach the
// world for the currently installed mock via `currentMock()`.
export function setMockCorrected(world: MockWorld, keys: string[]): void {
	world.config.corrected = keys.map((key) => ({ key, message: 'unusable value, default applied' }));
}

function isEmptyObject(value: unknown): boolean {
	return typeof value === 'object' && value !== null && !Array.isArray(value) && Object.keys(value).length === 0;
}

function valueAt(doc: unknown, path: string[]): unknown {
	return path.reduce<unknown>((node, part) => {
		if (typeof node !== 'object' || node === null || Array.isArray(node)) return undefined;
		return (node as Record<string, unknown>)[part];
	}, doc);
}

function withValueAt(node: unknown, path: string[], value: unknown): Record<string, unknown> {
	const [head, ...rest] = path;
	const base = typeof node === 'object' && node !== null && !Array.isArray(node) ? node : {};
	return {
		...base,
		[head]: rest.length === 0 ? value : withValueAt((base as Record<string, unknown>)[head], rest, value),
	};
}

// `key` is the dotted path from the section down, e.g. `manifold.fletcher`.
function reject(key: string, value: unknown): string | null {
	const path = key.split('.');
	// Real daemon rejects keys it doesn't recognize, per-key, rather than
	// silently ignoring or accepting them. `api.host` is NOT one of these —
	// it round-trips fine against a real daemon, unlike the read-only
	// treatment this mock used to give it.
	if (valueAt(CONFIG_DEFAULTS, path) === undefined) return `unknown setting "${key}"`;
	if (value === null) return null;
	if (key === 'logger.level' && !LEVELS.includes(String(value))) return 'unusable log level';
	if (key.startsWith('netbridge.ephemeral_port')) {
		const n = Number(value);
		if (!Number.isInteger(n) || n < 1 || n > 65535) return 'port out of range';
	}
	return null;
}

function differing(a: MockConfigDoc, b: MockConfigDoc): string[] {
	return flattenConfig(b).flatMap(({ key, value }) =>
		JSON.stringify(valueAt(a, key.split('.'))) === JSON.stringify(value) ? [] : [key]
	);
}

function view(world: MockWorld) {
	const { api: _api, ...runningWithoutApi } = world.config.running;
	return {
		running: runningWithoutApi,
		configured: world.config.configured,
		defaults: CONFIG_DEFAULTS,
		restart_required: differing(world.config.running, world.config.configured),
		corrected: world.config.corrected,
	};
}

export const configRoutes: Route[] = [
	{ method: 'GET', pattern: '/v0/config', fault: 'config', handler: (_req, world) => ok(view(world)) },
	{
		method: 'PATCH',
		pattern: '/v0/config',
		fault: 'config',
		handler: (req, world) => {
			const applied: string[] = [];
			const rejected: { key: string; message: string }[] = [];
			const body = (req.body ?? {}) as MockConfigDoc;

			// A patch names leaves, however deep: `{ manifold: { fletcher: { enabled } } }`
			// is the single key `manifold.fletcher.enabled`.
			for (const { key, value } of flattenConfig(body)) {
				if (isEmptyObject(value)) continue;
				const why = reject(key, value);
				if (why) {
					rejected.push({ key, message: why });
					continue;
				}
				const path = key.split('.');
				const fallback = valueAt(CONFIG_DEFAULTS, path);
				world.config.configured = withValueAt(
					world.config.configured,
					path,
					value === null ? fallback : value
				) as MockConfigDoc;
				applied.push(key);
			}

			if (applied.length === 0 && rejected.length > 0) {
				// Mirrors core exactly (confirmed against a real daemon,
				// origin/develop@56065f3): a patch where nothing at all
				// applied comes back on a 422 status with `success: false`
				// and `error: null`, yet `data` still carries the per-key
				// rejection reasons — they're real data, not an error.
				// Neither `ok()` (always forces `success: true`) nor `fail()`
				// (always forces `data: null`) can produce that shape, so the
				// Response is built directly here instead.
				return new Response(JSON.stringify({ success: false, error: null, data: { applied, rejected } }), {
					status: 422,
					headers: { 'content-type': 'application/json' },
				});
			}

			return ok({ applied, rejected });
		},
	},
];
