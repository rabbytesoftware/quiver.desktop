import type { JSX } from 'react';

import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render } from '@testing-library/react';

/** Renders `ui` as the root of a router that also knows the arrow details and app routes. */
export async function renderWithRouter(ui: JSX.Element) {
	const root = createRootRoute({ component: () => ui });
	const details = createRoute({ getParentRoute: () => root, path: '/arrow/$', component: () => null });
	const app = createRoute({ getParentRoute: () => root, path: '/app/$', component: () => null });
	const router = createRouter({
		routeTree: root.addChildren([details, app]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});
	await router.load();
	render(<RouterProvider router={router} />);
	return { router };
}
