import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { CollectionsScreen } from './collections-screen';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

async function renderScreen() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const collectionsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collections',
		component: CollectionsScreen,
	});
	const homeRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home-page" />,
	});
	const collectionRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collection/$',
		component: () => <div data-testid="collection-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([collectionsRoute, homeRoute, collectionRoute]),
		history: createMemoryHistory({ initialEntries: ['/collections'] }),
	});

	const view = render(
		<QueryClientProvider client={client}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	);
	await waitFor(() => expect(router.state.status).toBe('idle'));
	return view;
}

beforeEach(() => {
	vi.mocked(apiFetch).mockReset();
});

describe('CollectionsScreen', () => {
	it('shows the title, a count-aware subtitle, and every followed collection sorted alphabetically', async () => {
		vi.mocked(apiFetch).mockResolvedValue([
			{ namespace: 'guild/zulu-pack', name: 'Zulu Pack', description: 'Z.', arrow_count: 3, followed: true },
			{ namespace: 'guild/alpha-pack', name: 'Alpha Pack', description: 'A.', arrow_count: 5, followed: true },
		]);

		await renderScreen();

		expect(await screen.findByRole('heading', { name: 'Collections' })).toBeInTheDocument();
		expect(screen.getByText('2 followed')).toBeInTheDocument();
		expect(screen.getByText('5 arrows')).toBeInTheDocument();

		const html = document.body.innerHTML;
		expect(html.indexOf('Alpha Pack')).toBeLessThan(html.indexOf('Zulu Pack'));
	});

	it('navigates to a collection detail page when a tile is clicked', async () => {
		vi.mocked(apiFetch).mockResolvedValue([
			{
				namespace: 'guild/frosthold-pack',
				name: 'Frosthold Pack',
				description: 'Survival.',
				arrow_count: 14,
				followed: true,
			},
		]);

		await renderScreen();

		fireEvent.click(await screen.findByRole('link', { name: /frosthold pack/i }));
		expect(await screen.findByTestId('collection-page')).toBeInTheDocument();
	});

	it('navigates back to Home from the breadcrumb', async () => {
		vi.mocked(apiFetch).mockResolvedValue([]);
		await renderScreen();
		fireEvent.click(screen.getByRole('link', { name: /home/i }));
		expect(await screen.findByTestId('home-page')).toBeInTheDocument();
	});

	it('shows a zero-state subtitle without erroring when nothing is followed yet', async () => {
		vi.mocked(apiFetch).mockResolvedValue([]);
		await renderScreen();
		expect(await screen.findByText('0 followed')).toBeInTheDocument();
	});
});
