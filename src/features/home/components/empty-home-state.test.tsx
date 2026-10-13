import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { EmptyHomeState } from './empty-home-state';

async function renderState() {
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: EmptyHomeState });
	const searchRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/search',
		component: () => <div data-testid="search-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([homeRoute, searchRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	const view = render(<RouterProvider router={router} />);
	await waitFor(() => expect(router.state.status).toBe('idle'));
	return view;
}

describe('EmptyHomeState', () => {
	it('shows the heading and description', async () => {
		await renderState();
		expect(screen.getByText('Nothing here yet')).toBeInTheDocument();
		expect(
			screen.getByText('Installed arrows and the collections you follow will show up here.')
		).toBeInTheDocument();
	});

	it('navigates to /search when the CTA is clicked', async () => {
		await renderState();
		fireEvent.click(screen.getByRole('link', { name: /search for arrows/i }));
		expect(await screen.findByTestId('search-page')).toBeInTheDocument();
	});
});
