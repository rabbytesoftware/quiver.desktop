import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { useSearchStore } from '@/lib/core-store/store/search';
import { createMockBackend, type MockRuntime } from '@/lib/mock';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { ResultsScreen } from './results-screen';

let mock: MockRuntime;

beforeEach(() => {
	mock = createMockBackend('normal');
	installBackend(mock.backend);
	useSearchStore.getState().reset();
});

afterEach(() => {
	mock.dispose();
	resetBackend();
});

// ArrowCard renders a router Link, so ResultsScreen needs a real router in
// scope -- mirrors the harness in result-grid.test.tsx.
function renderScreen(query: string) {
	const rootRoute = createRootRoute({
		component: () => <ResultsScreen query={query} />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	return render(<RouterProvider router={router} />);
}

describe('ResultsScreen', () => {
	it('shows local results without waiting for the network lane', async () => {
		renderScreen('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
		expect(screen.getByRole('heading', { name: 'minecraft' })).toBeInTheDocument();
	});

	it('renders nothing but the empty frame for an empty query', () => {
		renderScreen('');
		expect(screen.queryByRole('heading')).not.toBeInTheDocument();
		expect(screen.queryAllByRole('link')).toHaveLength(0);
	});
});
