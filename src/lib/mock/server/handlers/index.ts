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
		handler: () => bareHealth(),
	},
];

/** Order is not load-bearing: patterns must agree on segment count before any
 *  segment is compared, so no route can shadow another at a different depth. */
export const ALL_ROUTES: Route[] = [
	...healthRoutes,
	...arrowRoutes,
	...runtimeRoutes,
	...collectionRoutes,
	...searchRoutes,
];
