import { MOCK_COMMANDS, MOCK_CORE_VERSIONS } from '../../console';
import { ok } from '../envelope';
import type { Route } from '../router';

// The console's two plain-HTTP reads. The stream and the exec call are not
// routes: a response here is returned whole, and those two are not.
export const consoleRoutes: Route[] = [
	{ method: 'GET', pattern: '/versions', fault: 'console', handler: () => ok(MOCK_CORE_VERSIONS) },
	{
		method: 'GET',
		pattern: '/v0/console/commands',
		fault: 'console',
		handler: () => ok({ commands: MOCK_COMMANDS }),
	},
];
