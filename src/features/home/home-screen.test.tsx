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

import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';
import { apiFetch } from '@/lib/transport/api';

import { HomeScreen } from './home-screen';
import { useArrowStore } from '../../lib/core-store/store/arrows';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

const catalogRecord = (overrides: Partial<ArrowCatalogRecord> & { namespace: string }): ArrowCatalogRecord => ({
	connectionId: 'local',
	name: overrides.namespace,
	description: `Description for ${overrides.namespace}`,
	tags: [],
	icon: null,
	banner: null,
	version: '1.0.0',
	...overrides,
});

function mockCollectionsResponse(items: unknown[] = []) {
	vi.mocked(apiFetch).mockImplementation((path: string) => {
		if (path.startsWith('/v0/collection')) return Promise.resolve(items);
		return Promise.resolve(undefined);
	});
}

async function renderHome() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: HomeScreen });
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});
	const collectionRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collection/$',
		component: () => <div data-testid="collection-page" />,
	});
	const libraryRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/library',
		component: () => <div data-testid="library-page" />,
	});
	const collectionsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collections',
		component: () => <div data-testid="collections-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, arrowRoute, collectionRoute, libraryRoute, collectionsRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
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
	useArrowStore.getState().reset();
	vi.mocked(apiFetch).mockReset();
	mockCollectionsResponse([]);
});

describe('HomeScreen', () => {
	it('shows the empty state once both the catalog and collections have finished loading with nothing in either', async () => {
		useArrowStore.getState().setCatalog([]);
		await renderHome();
		expect(await screen.findByText('Nothing here yet')).toBeInTheDocument();
	});

	it('does not show the empty state while the catalog is still loading', async () => {
		await renderHome();
		expect(screen.queryByText('Nothing here yet')).not.toBeInTheDocument();
	});

	it('omits the Recents section entirely when no arrow has ever been used', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		await renderHome();
		await screen.findByText('Library');
		expect(screen.queryByText('Recents')).not.toBeInTheDocument();
	});

	it('shows Recents sorted most-recently-used first, capped at three', async () => {
		useArrowStore
			.getState()
			.setCatalog([
				catalogRecord({ namespace: 'a@1', name: 'Alpha', last_used_at: '2026-07-01T00:00:00Z' }),
				catalogRecord({ namespace: 'b@1', name: 'Bravo', last_used_at: '2026-07-03T00:00:00Z' }),
				catalogRecord({ namespace: 'c@1', name: 'Charlie', last_used_at: '2026-07-02T00:00:00Z' }),
				catalogRecord({ namespace: 'd@1', name: 'Delta', last_used_at: '2026-07-04T00:00:00Z' }),
			]);
		await renderHome();

		await screen.findByText('Recents');
		const recentsHeading = screen.getByText('Recents');
		const recentsSection = recentsHeading.closest('section')!;
		const html = recentsSection.innerHTML;
		const indexOf = (name: string) => html.indexOf(name);

		expect(indexOf('Alpha')).toBe(-1);
		expect(indexOf('Delta')).toBeGreaterThan(-1);
		expect(indexOf('Delta')).toBeLessThan(indexOf('Bravo'));
		expect(indexOf('Bravo')).toBeLessThan(indexOf('Charlie'));
	});

	it('shows the Library section sorted alphabetically with a working "view all" link', async () => {
		useArrowStore
			.getState()
			.setCatalog([
				catalogRecord({ namespace: 'z@1', name: 'Zulu' }),
				catalogRecord({ namespace: 'a@1', name: 'Alpha' }),
			]);
		await renderHome();

		await screen.findByText('Library');
		const libraryHeading = screen.getByText('Library');
		const librarySection = libraryHeading.closest('section')!;
		const alphaIndex = librarySection.innerHTML.indexOf('Alpha');
		const zuluIndex = librarySection.innerHTML.indexOf('Zulu');
		expect(alphaIndex).toBeGreaterThan(-1);
		expect(alphaIndex).toBeLessThan(zuluIndex);

		fireEvent.click(screen.getByRole('link', { name: /view all 2 arrows/i }));
		expect(await screen.findByTestId('library-page')).toBeInTheDocument();
	});

	it('shows the Collections section from useFollowedCollections with a working "view all" link', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		mockCollectionsResponse([
			{
				namespace: 'guild/frosthold-pack',
				name: 'Frosthold Pack',
				description: 'Survival essentials.',
				arrow_count: 14,
				followed: true,
			},
		]);
		await renderHome();

		// "Frosthold Pack" legitimately renders twice per tile (the drawn-banner
		// fallback name, and the always-visible caption below it) -- assert
		// presence via getAllByText rather than the ambiguous getByText.
		expect((await screen.findAllByText('Frosthold Pack')).length).toBeGreaterThan(0);
		expect(screen.getByText('14 arrows')).toBeInTheDocument();

		fireEvent.click(screen.getByRole('link', { name: /view 1 collection/i }));
		expect(await screen.findByTestId('collections-page')).toBeInTheDocument();
	});

	it('resolves a detached arrow via useStop when its badge is activated, without navigating', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'detached', active_run: null, last_return: null });
		await renderHome();

		const badge = await screen.findByRole('button', { name: 'Detached' });
		fireEvent.click(badge);

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/a%401/stop', expect.objectContaining({ method: 'POST' }))
		);
		expect(screen.queryByTestId('arrow-page')).not.toBeInTheDocument();
	});

	it('resolves a detached arrow from within the Recents section too', async () => {
		useArrowStore
			.getState()
			.setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha', last_used_at: '2026-07-01T00:00:00Z' })]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'detached', active_run: null, last_return: null });
		await renderHome();

		const recentsSection = (await screen.findByText('Recents')).closest('section')!;
		const badge = await within(recentsSection).findByRole('button', { name: 'Detached' });
		fireEvent.click(badge);

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/a%401/stop', expect.objectContaining({ method: 'POST' }))
		);
	});
});
