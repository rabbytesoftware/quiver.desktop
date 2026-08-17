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

function reject(section: string, name: string, value: unknown): string | null {
	const key = `${section}.${name}`;
	// Real daemon rejects keys it doesn't recognize, per-key, rather than
	// silently ignoring or accepting them. `api.host` is NOT one of these —
	// it round-trips fine against a real daemon, unlike the read-only
	// treatment this mock used to give it.
	if (!(name in (CONFIG_DEFAULTS[section] ?? {}))) return `unknown setting "${key}"`;
	if (value === null) return null;
	if (key === 'logger.level' && !LEVELS.includes(String(value))) return 'unusable log level';
	if (key.startsWith('netbridge.ephemeral_port')) {
		const n = Number(value);
		if (!Number.isInteger(n) || n < 1 || n > 65535) return 'port out of range';
	}
	return null;
}

function differing(a: MockConfigDoc, b: MockConfigDoc): string[] {
	const keys: string[] = [];
	for (const section of Object.keys(b)) {
		for (const [name, value] of Object.entries(b[section] ?? {})) {
			if (JSON.stringify(a[section]?.[name]) !== JSON.stringify(value)) keys.push(`${section}.${name}`);
		}
	}
	return keys;
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

			for (const [section, settings] of Object.entries(body)) {
				for (const [name, value] of Object.entries(settings ?? {})) {
					const key = `${section}.${name}`;
					const why = reject(section, name, value);
					if (why) {
						rejected.push({ key, message: why });
						continue;
					}
					const fallback = CONFIG_DEFAULTS[section]?.[name];
					world.config.configured[section] = {
						...world.config.configured[section],
						[name]: value === null ? fallback : value,
					};
					applied.push(key);
				}
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
