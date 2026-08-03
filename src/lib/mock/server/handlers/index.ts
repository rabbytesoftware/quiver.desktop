import { bareHealth } from '../envelope';
import type { Route } from '../router';
import { arrowRoutes } from './arrow';
import { collectionRoutes } from './collection';
import { runtimeRoutes } from './runtime';
import { searchRoutes } from './search';

const healthRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/health',
		fault: 'health',
		// Unenveloped, matching core. `coreIsReachable` judges this on the HTTP
		// status alone and never parses it.
		handler: () => bareHealth(),
	},
];

/**
 * Order is not load-bearing here, and that is a property of the matcher rather
 * than luck: patterns must agree on segment COUNT before any segment is
 * compared, so `/v0/search` (two) and `/v0/search/discover` (three) can never
 * shadow each other, and no `:capture` can swallow a literal at a different
 * depth. The one genuine ambiguity — `/v0/arrow/:ns` versus a literal route at
 * the same depth — does not exist, because every three-segment arrow route is a
 * capture.
 */
export const ALL_ROUTES: Route[] = [
	...healthRoutes,
	...arrowRoutes,
	...runtimeRoutes,
	...collectionRoutes,
	...searchRoutes,
];
