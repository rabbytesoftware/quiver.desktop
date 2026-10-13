import { ok } from '../envelope';
import type { Route } from '../router';

// Setup flips `configured` only. `on_path` reports the daemon's own running
// environment, which a rc-file edit cannot change until it restarts.
export const pathRoutes: Route[] = [
	{ method: 'GET', pattern: '/v0/system/path', fault: 'path', handler: (_req, world) => ok(world.path) },
	{
		method: 'POST',
		pattern: '/v0/system/path',
		fault: 'path',
		handler: (_req, world) => {
			world.path.configured = true;
			return ok(world.path);
		},
	},
];
