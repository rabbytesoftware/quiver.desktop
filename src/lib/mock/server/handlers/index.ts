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

export const ALL_ROUTES: Route[] = [
	...healthRoutes,
	...arrowRoutes,
	...runtimeRoutes,
	...collectionRoutes,
	...searchRoutes,
];
