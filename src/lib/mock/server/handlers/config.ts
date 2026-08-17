import { ok } from '../envelope';
import type { Route } from '../router';

const DEFAULTS = {
	netbridge: { enabled: true, ephemeral_port_start: 49152, ephemeral_port_end: 65535 },
	api: { host: 'unix://' },
	logger: { enabled: true, level: 'info' },
	manifold: { fetch_timeout: '30s' },
	vault: { sweep_interval: '5m', ttl: '24h', index_ttl: '24h' },
	arrows: { auto_retry: { enabled: true, retries: 3 } },
	search: { per_provider_limit: 25, fetch_concurrency: 4, provider_timeout: '10s' },
} as const;

const LEVELS = ['debug', 'trace', 'info', 'warn', 'warning', 'error', 'fatal', 'panic'];

type Doc = Record<string, Record<string, unknown>>;

const clone = (): Doc => JSON.parse(JSON.stringify(DEFAULTS)) as Doc;

// The daemon boots from `configured` and never re-reads it, so `running` is a
// snapshot taken at install time and only a restart moves it.
let running: Doc = clone();
let configured: Doc = clone();

// Damage already in the file, which only the daemon can see. Settable so the
// panel's `corrected` notice is reachable from a test.
let corrected: { key: string; message: string }[] = [];

export function resetMockConfig(): void {
	running = clone();
	configured = clone();
	corrected = [];
}

export function setMockCorrected(keys: string[]): void {
	corrected = keys.map((key) => ({ key, message: 'unusable value, default applied' }));
}

function reject(key: string, value: unknown): string | null {
	if (key === 'logger.level' && !LEVELS.includes(String(value))) return 'unusable log level';
	if (key.startsWith('netbridge.ephemeral_port')) {
		const n = Number(value);
		if (!Number.isInteger(n) || n < 1 || n > 65535) return 'port out of range';
	}
	if (key === 'api.host') return 'read-only';
	return null;
}

function differing(a: Doc, b: Doc): string[] {
	const keys: string[] = [];
	for (const section of Object.keys(b)) {
		for (const [name, value] of Object.entries(b[section] ?? {})) {
			if (JSON.stringify(a[section]?.[name]) !== JSON.stringify(value)) keys.push(`${section}.${name}`);
		}
	}
	return keys;
}

function view() {
	const { api: _api, ...runningWithoutApi } = running;
	return {
		running: runningWithoutApi,
		configured,
		defaults: DEFAULTS,
		restart_required: differing(running, configured),
		corrected,
	};
}

export const configRoutes: Route[] = [
	{ method: 'GET', pattern: '/v0/config', fault: 'config', handler: () => ok(view()) },
	{
		method: 'PATCH',
		pattern: '/v0/config',
		fault: 'config',
		handler: (req) => {
			const applied: string[] = [];
			const rejected: { key: string; message: string }[] = [];
			const body = (req.body ?? {}) as Doc;

			for (const [section, settings] of Object.entries(body)) {
				for (const [name, value] of Object.entries(settings ?? {})) {
					const key = `${section}.${name}`;
					const why = value === null ? null : reject(key, value);
					if (why) {
						rejected.push({ key, message: why });
						continue;
					}
					const fallback = (DEFAULTS as unknown as Doc)[section]?.[name];
					configured[section] = { ...configured[section], [name]: value === null ? fallback : value };
					applied.push(key);
				}
			}

			return ok({ applied, rejected });
		},
	},
];
