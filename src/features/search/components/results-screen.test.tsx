import { StrictMode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { IDLE_BEFORE_PASS_MS } from '@/lib/core-store/search/pass';
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

	// Mirrors pass.test.ts's "does not outlive dispose" at the screen level:
	// unmounting is the cancel, so a pass in flight must not keep mutating the
	// store once its screen is gone.
	it('does not let a pass outlive the screen that started it', async () => {
		vi.useFakeTimers();
		try {
			const { unmount } = renderScreen('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
			expect(useSearchStore.getState().phase).toBe('discovering');

			unmount();
			const before = useSearchStore.getState();
			await vi.advanceTimersByTimeAsync(30_000);
			expect(useSearchStore.getState()).toEqual(before);
		} finally {
			vi.useRealTimers();
		}
	});
});

// Mirrors main.tsx exactly: StrictMode wraps QueryClientProvider wraps
// RouterProvider, all above this screen. StrictMode nested any lower (e.g.
// directly around ResultsScreen, below the router) does not reproduce the
// bug this guards against -- the router's own effects are part of what
// double-invokes at the real root.
function renderAtAppRoot(query: string) {
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

	const queryClient = new QueryClient();

	return render(
		<StrictMode>
			<QueryClientProvider client={queryClient}>
				<RouterProvider router={router} />
			</QueryClientProvider>
		</StrictMode>
	);
}

describe('ResultsScreen mounted at the real app root', () => {
	it('still starts a search under StrictMode, not just in a plain render', async () => {
		renderAtAppRoot('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
	});
});
