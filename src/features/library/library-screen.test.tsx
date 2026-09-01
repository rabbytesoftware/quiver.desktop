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

import { useArrowStore } from '@/lib/core-store/store/arrows';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';
import { apiFetch } from '@/lib/transport/api';

import { LibraryScreen } from './library-screen';

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

async function renderScreen() {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const libraryRoute = createRoute({ getParentRoute: () => rootRoute, path: '/library', component: LibraryScreen });
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
		routeTree: rootRoute.addChildren([libraryRoute, homeRoute, arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/library'] }),
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
	vi.mocked(apiFetch).mockResolvedValue(undefined);
});

describe('LibraryScreen', () => {
	it('shows the title and a count-aware subtitle', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		await renderScreen();
		expect(screen.getByRole('heading', { name: 'Library' })).toBeInTheDocument();
		expect(screen.getByText('1 installed arrow')).toBeInTheDocument();
	});

	it('pluralizes the subtitle for more than one arrow', async () => {
		useArrowStore
			.getState()
			.setCatalog([
				catalogRecord({ namespace: 'a@1', name: 'Alpha' }),
				catalogRecord({ namespace: 'b@1', name: 'Bravo' }),
			]);
		await renderScreen();
		expect(screen.getByText('2 installed arrows')).toBeInTheDocument();
	});

	it('lists every arrow in the store, sorted alphabetically', async () => {
		useArrowStore
			.getState()
			.setCatalog([
				catalogRecord({ namespace: 'z@1', name: 'Zulu' }),
				catalogRecord({ namespace: 'a@1', name: 'Alpha' }),
			]);
		await renderScreen();
		const html = document.body.innerHTML;
		expect(html.indexOf('Alpha')).toBeLessThan(html.indexOf('Zulu'));
	});

	it('navigates back to Home from the breadcrumb', async () => {
		useArrowStore.getState().setCatalog([]);
		await renderScreen();
		fireEvent.click(screen.getByRole('link', { name: /home/i }));
		expect(await screen.findByTestId('home-page')).toBeInTheDocument();
	});

	it('resolves a detached arrow via useStop when its badge is activated', async () => {
		useArrowStore.getState().setCatalog([catalogRecord({ namespace: 'a@1', name: 'Alpha' })]);
		useArrowStore
			.getState()
			.applyRuntimeUpdate({ namespace: 'a@1', state: 'detached', active_run: null, last_return: null });
		await renderScreen();

		fireEvent.click(screen.getByRole('button', { name: 'Detached' }));

		await waitFor(() =>
			expect(apiFetch).toHaveBeenCalledWith('/v0/runtime/a%401/stop', expect.objectContaining({ method: 'POST' }))
		);
	});
});
