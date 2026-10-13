import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiFetch } from '@/lib/transport/api';

import { RecommendedScreen } from './recommended-screen';

vi.mock('@/lib/transport/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('@/lib/transport/api')>()),
	apiFetch: vi.fn(),
}));

function recommendation(namespace: string, overrides: Record<string, unknown> = {}) {
	return {
		namespace,
		name: namespace,
		description: `About ${namespace}`,
		tags: [],
		versions: ['v1'],
		compatible_os: [],
		installed: false,
		known: true,
		stars: 1,
		...overrides,
	};
}

function shelf(arrows: unknown[], overrides: Record<string, unknown> = {}) {
	return { id: 'popular', title: 'Popular', refreshed_at: '2026-09-30T12:00:00Z', arrows, ...overrides };
}

function mockHome(home: unknown) {
	vi.mocked(apiFetch).mockImplementation((path: string) => {
		if (path === '/v0/home') return home instanceof Error ? Promise.reject(home) : Promise.resolve(home);
		return Promise.resolve(undefined);
	});
}

async function renderScreen() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const recommendedRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/recommended',
		component: RecommendedScreen,
	});
	const homeRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home-page" />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([recommendedRoute, homeRoute, arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/recommended'] }),
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
	mockHome({ shelves: [], refreshing: false });
});

describe('RecommendedScreen', () => {
	it('lists every shelf in full, each under its own title, with no host label', async () => {
		const many = ['One', 'Two', 'Three', 'Four', 'Five'].map((name) =>
			recommendation(`github.com/x/${name.toLowerCase()}`, { name, source: 'github' })
		);
		mockHome({
			shelves: [
				shelf(many, { id: 'popular', title: 'Popular' }),
				shelf([recommendation('github.com/x/fresh', { name: 'Fresh' })], {
					id: 'recent',
					title: 'Recently updated',
				}),
			],
			refreshing: false,
		});
		await renderScreen();

		const popular = (await screen.findByText('Popular')).closest('section')!;
		expect(within(popular).getAllByRole('link')).toHaveLength(5);
		const recent = screen.getByText('Recently updated').closest('section')!;
		expect(within(recent).getAllByText('Fresh').length).toBeGreaterThan(0);
		expect(screen.queryByText(/^github$/i)).not.toBeInTheDocument();
		expect(screen.getByText('6 recommended arrows')).toBeInTheDocument();
	});

	it('hides arrows the user already holds and drops a shelf left empty', async () => {
		mockHome({
			shelves: [
				shelf([
					recommendation('github.com/x/mine', { name: 'Mine', installed: true }),
					recommendation('github.com/x/new', { name: 'Newbie' }),
				]),
				shelf([recommendation('github.com/x/curated', { name: 'Curated', provenance: 'collection' })], {
					id: 'other',
					title: 'Other shelf',
				}),
			],
			refreshing: false,
		});
		await renderScreen();

		expect((await screen.findAllByText('Newbie')).length).toBeGreaterThan(0);
		expect(screen.queryByText('Mine')).not.toBeInTheDocument();
		expect(screen.queryByText('Curated')).not.toBeInTheDocument();
		expect(screen.queryByText('Other shelf')).not.toBeInTheDocument();
	});

	it('shows a plain empty message for an empty idle snapshot', async () => {
		mockHome({ shelves: [shelf([])], refreshing: false });
		await renderScreen();

		expect(await screen.findByText('No recommendations right now.')).toBeInTheDocument();
		expect(document.querySelector('[data-slot="card-skeleton"]')).not.toBeInTheDocument();
	});

	it('shows a skeleton row, not the empty message, for an empty snapshot that is still refreshing', async () => {
		mockHome({ shelves: [shelf([])], refreshing: true });
		await renderScreen();

		await waitFor(() => expect(document.querySelectorAll('[data-slot="card-skeleton"]').length).toBeGreaterThan(0));
		expect(screen.queryByText('No recommendations right now.')).not.toBeInTheDocument();
	});

	it('shows the empty message for a core without the endpoint', async () => {
		mockHome(new ApiError('not found', 404));
		await renderScreen();

		expect(await screen.findByText('No recommendations right now.')).toBeInTheDocument();
	});

	it('does not show the empty message while the snapshot is still loading', async () => {
		vi.mocked(apiFetch).mockImplementation(() => new Promise(() => {}));
		await renderScreen();

		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith('/v0/home'));
		expect(screen.queryByText('No recommendations right now.')).not.toBeInTheDocument();
	});

	it('links back to Home', async () => {
		await renderScreen();

		fireEvent.click(await screen.findByRole('link', { name: 'Home' }));
		expect(await screen.findByTestId('home-page')).toBeInTheDocument();
	});
});
