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

import { QUIVER_CORE_NAMESPACE, QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { useCatalogVisibilityStore } from '@/features/settings/stores/catalog-visibility-store';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';
import { ApiError, apiFetch } from '@/lib/transport/api';

import { HomeScreen } from './home-screen';
import { useArrowStore } from '../../lib/core-store/store/arrows';

vi.mock('@/lib/transport/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('@/lib/transport/api')>()),
	apiFetch: vi.fn(),
}));

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

const EMPTY_HOME = { shelves: [], refreshing: false };

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

function mockCollectionsResponse(items: unknown[] = [], home: unknown = EMPTY_HOME) {
	vi.mocked(apiFetch).mockImplementation((path: string) => {
		if (path.startsWith('/v0/collection')) return Promise.resolve(items);
		if (path === '/v0/home') {
			return home instanceof Error ? Promise.reject(home) : Promise.resolve(home);
		}
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
	const recommendedRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/recommended',
		component: () => <div data-testid="recommended-page" />,
	});
	const collectionsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collections',
		component: () => <div data-testid="collections-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([
			indexRoute,
			arrowRoute,
			collectionRoute,
			libraryRoute,
			recommendedRoute,
			collectionsRoute,
		]),
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
	useCatalogVisibilityStore.setState({ showSelfComponents: true });
	vi.mocked(apiFetch).mockReset();
	mockCollectionsResponse([]);
});

describe('HomeScreen auto-made badge', () => {
	it('marks an inferred arrow in the library and no other', async () => {
		useArrowStore
			.getState()
			.setCatalog([
				catalogRecord({ namespace: 'a@1', name: 'Alpha', origin: 'inferred', confidence: 'medium' }),
				catalogRecord({ namespace: 'b@1', name: 'Bravo', origin: 'declared' }),
			]);
		await renderHome();
		await screen.findByText('Library');
		expect(screen.getAllByText('Auto-made')).toHaveLength(1);
	});
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

	it('shows Quiver’s own rows in the Library section by default', async () => {
		useArrowStore
			.getState()
			.setCatalog([catalogRecord({ namespace: `${QUIVER_DESKTOP_NAMESPACE}@stable-1.0`, name: 'Quiver' })]);
		await renderHome();

		// "Quiver" legitimately renders twice per tile (the drawn-banner
		// fallback name, and the always-visible caption below it) -- same
		// reasoning as the "Frosthold Pack" collection assertion above.
		expect((await screen.findAllByText('Quiver')).length).toBeGreaterThan(0);
	});

	it('excludes both of Quiver’s own self-registered rows from Library and Recents once the setting is off', async () => {
		useCatalogVisibilityStore.getState().setShowSelfComponents(false);
		useArrowStore.getState().setCatalog([
			catalogRecord({
				namespace: `${QUIVER_DESKTOP_NAMESPACE}@stable-1.0`,
				name: 'Quiver Desktop',
				last_used_at: '2026-07-01T00:00:00Z',
			}),
			catalogRecord({
				namespace: `${QUIVER_CORE_NAMESPACE}@stable-1.0`,
				name: 'Quiver Core',
				last_used_at: '2026-07-02T00:00:00Z',
			}),
			catalogRecord({ namespace: 'a@1', name: 'Alpha' }),
		]);
		await renderHome();

		await waitFor(() => expect(screen.getAllByText('Alpha').length).toBeGreaterThan(0));
		expect(screen.queryByText('Quiver Desktop')).not.toBeInTheDocument();
		expect(screen.queryByText('Quiver Core')).not.toBeInTheDocument();
	});
});

describe('HomeScreen recommendations', () => {
	it('shows the shelf under its own title, above Library, with no host label', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		mockCollectionsResponse([], {
			shelves: [shelf([recommendation('github.com/x/tool', { name: 'Tool', source: 'github' })])],
			refreshing: false,
		});
		await renderHome();

		const heading = await screen.findByText('Popular');
		const section = heading.closest('section')!;
		expect(within(section).getAllByText('Tool').length).toBeGreaterThan(0);
		expect(screen.queryByText(/^github$/i)).not.toBeInTheDocument();
		const library = screen.getByText('Library').closest('section')!;
		expect(section.compareDocumentPosition(library) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(within(section).getByRole('link')).toHaveAttribute('href', '/arrow/github.com/x/tool');
	});

	it('puts the first shelf above Library and every later shelf below it', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		mockCollectionsResponse([], {
			shelves: [
				shelf([recommendation('github.com/x/one', { name: 'One' })], { id: 'lead', title: 'Lead shelf' }),
				shelf([recommendation('github.com/x/two', { name: 'Two' })], { id: 'later', title: 'Later shelf' }),
			],
			refreshing: false,
		});
		await renderHome();

		const lead = (await screen.findByText('Lead shelf')).closest('section')!;
		const library = screen.getByText('Library').closest('section')!;
		const later = screen.getByText('Later shelf').closest('section')!;
		expect(lead.compareDocumentPosition(library) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(library.compareDocumentPosition(later) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
	});

	it('shows the shelf instead of the empty state on a fresh install', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], { shelves: [shelf([recommendation('github.com/x/tool')])], refreshing: false });
		await renderHome();

		expect(await screen.findByText('Popular')).toBeInTheDocument();
		expect(screen.queryByText('Nothing here yet')).not.toBeInTheDocument();
	});

	it('hides arrows the user already holds, installed or via a followed collection', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], {
			shelves: [
				shelf([
					recommendation('github.com/x/mine', { name: 'Mine', installed: true }),
					recommendation('github.com/x/curated', { name: 'Curated', provenance: 'collection' }),
					recommendation('github.com/x/new', { name: 'Newbie' }),
				]),
			],
			refreshing: false,
		});
		await renderHome();

		expect((await screen.findAllByText('Newbie')).length).toBeGreaterThan(0);
		expect(screen.queryByText('Mine')).not.toBeInTheDocument();
		expect(screen.queryByText('Curated')).not.toBeInTheDocument();
	});

	it('drops a shelf whose arrows are all held and falls back to the empty state', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], {
			shelves: [shelf([recommendation('github.com/x/mine', { installed: true })])],
			refreshing: false,
		});
		await renderHome();

		expect(await screen.findByText('Nothing here yet')).toBeInTheDocument();
		expect(screen.queryByText('Popular')).not.toBeInTheDocument();
	});

	it('shows a skeleton row, not the empty state, for an empty snapshot that is still refreshing', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], { shelves: [shelf([])], refreshing: true });
		await renderHome();

		expect(await screen.findByText('Recommended')).toBeInTheDocument();
		expect(document.querySelectorAll('[data-slot="card-skeleton"]').length).toBeGreaterThan(0);
		expect(screen.queryByText('Nothing here yet')).not.toBeInTheDocument();
	});

	it('shows the empty state for an empty snapshot that is not refreshing', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], { shelves: [shelf([])], refreshing: false });
		await renderHome();

		expect(await screen.findByText('Nothing here yet')).toBeInTheDocument();
		expect(document.querySelector('[data-slot="card-skeleton"]')).not.toBeInTheDocument();
	});

	it('does not show the empty state while the snapshot is still loading', async () => {
		useArrowStore.getState().setCatalog([]);
		vi.mocked(apiFetch).mockImplementation((path: string) =>
			path === '/v0/home' ? new Promise(() => {}) : Promise.resolve([])
		);
		await renderHome();

		await waitFor(() => expect(apiFetch).toHaveBeenCalledWith('/v0/home'));
		expect(screen.queryByText('Nothing here yet')).not.toBeInTheDocument();
	});

	it('treats a core without the endpoint as no shelf, with the usual empty state', async () => {
		useArrowStore.getState().setCatalog([]);
		mockCollectionsResponse([], new ApiError('not found', 404));
		await renderHome();

		expect(await screen.findByText('Nothing here yet')).toBeInTheDocument();
	});

	it('keeps Library when the snapshot is empty and idle', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		await renderHome();

		await screen.findByText('Library');
		expect(screen.queryByText('Recommended')).not.toBeInTheDocument();
	});

	it('caps the lead shelf at three arrows and links to the full list with the count', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		const arrows = ['One', 'Two', 'Three', 'Four', 'Five'].map((name) =>
			recommendation(`github.com/x/${name.toLowerCase()}`, { name })
		);
		mockCollectionsResponse([], { shelves: [shelf(arrows)], refreshing: false });
		await renderHome();

		const section = (await screen.findByText('Popular')).closest('section')!;
		expect(within(section).getAllByRole('link', { name: /^(One|Two|Three)/ })).toHaveLength(3);
		expect(within(section).queryByText('Four')).not.toBeInTheDocument();
		expect(within(section).queryByText('Five')).not.toBeInTheDocument();

		const link = within(section).getByRole('link', { name: 'View all 5' });
		expect(link).toHaveAttribute('href', '/recommended');
		fireEvent.click(link);
		expect(await screen.findByTestId('recommended-page')).toBeInTheDocument();
	});

	it('counts only arrows the user does not already hold in the lead shelf link', async () => {
		useArrowStore.getState().setCatalog([]);
		const arrows = [
			...['One', 'Two', 'Three', 'Four'].map((name) =>
				recommendation(`github.com/x/${name.toLowerCase()}`, { name })
			),
			recommendation('github.com/x/mine', { name: 'Mine', installed: true }),
		];
		mockCollectionsResponse([], { shelves: [shelf(arrows)], refreshing: false });
		await renderHome();

		expect(await screen.findByRole('link', { name: 'View all 4' })).toBeInTheDocument();
	});

	it('shows no link when the lead shelf has three arrows or fewer', async () => {
		useArrowStore.getState().setCatalog([]);
		const arrows = ['One', 'Two', 'Three'].map((name) =>
			recommendation(`github.com/x/${name.toLowerCase()}`, { name })
		);
		mockCollectionsResponse([], { shelves: [shelf(arrows)], refreshing: false });
		await renderHome();

		const section = (await screen.findByText('Popular')).closest('section')!;
		expect(within(section).getAllByRole('link')).toHaveLength(3);
		expect(screen.queryByRole('link', { name: /view all/i })).not.toBeInTheDocument();
	});

	it('leaves trailing shelves uncapped', async () => {
		useArrowStore.getState().setCatalog([]);
		const arrows = ['One', 'Two', 'Three', 'Four', 'Five'].map((name) =>
			recommendation(`github.com/x/${name.toLowerCase()}`, { name })
		);
		mockCollectionsResponse([], {
			shelves: [
				shelf([recommendation('github.com/x/lead', { name: 'Lead' })], { id: 'lead', title: 'Lead shelf' }),
				shelf(arrows, { id: 'later', title: 'Later shelf' }),
			],
			refreshing: false,
		});
		await renderHome();

		const later = (await screen.findByText('Later shelf')).closest('section')!;
		expect(within(later).getAllByRole('link')).toHaveLength(5);
	});
});
