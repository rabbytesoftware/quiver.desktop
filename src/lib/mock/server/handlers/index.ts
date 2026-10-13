import { bareHealth } from '../envelope';
import type { Route } from '../router';
import { arrowRoutes } from './arrow';
import { collectionRoutes } from './collection';
import { configRoutes } from './config';
import { consoleRoutes } from './console';
import { homeRoutes } from './home';
import { pathRoutes } from './path';
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
	...homeRoutes,
	...configRoutes,
	...pathRoutes,
	...consoleRoutes,
];
