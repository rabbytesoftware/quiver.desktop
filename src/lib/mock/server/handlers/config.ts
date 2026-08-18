import { ok } from '../envelope';
import type { Route } from '../router';

/**
 * The daemon configuration, seen three ways at once: what the process is using,
 * what the next start will use, and what ships in the binary. `running` carries
 * no api section -- a --host flag can override the configured bind address at
 * start, so the daemon cannot vouch for one from configuration alone.
 */
const SEARCH_DEFAULTS = {
	per_provider_limit: 25,
	fetch_concurrency: 8,
	provider_timeout: '10s',
};

const CONFIG = {
	netbridge: { enabled: true },
	logger: { level: 'info' },
	manifold: { registry: 'https://registry.quiver.sh' },
	vault: { path: '~/.quiver/vault' },
	arrows: { auto_retry: { enabled: true, retries: 2 } },
	search: SEARCH_DEFAULTS,
};

export const configRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/config',
		fault: 'config',
		handler: () =>
			ok({
				running: CONFIG,
				configured: { ...CONFIG, api: { host: '127.0.0.1', port: 7700 } },
				defaults: { ...CONFIG, api: { host: '127.0.0.1', port: 7700 } },
				restart_required: [],
				corrected: [],
			}),
	},
];
