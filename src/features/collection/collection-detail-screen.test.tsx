import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';

import { apiFetch } from '@/lib/transport/api';

import { CollectionDetailScreen } from './collection-detail-screen';

vi.mock('@/lib/transport/api', () => ({ apiFetch: vi.fn() }));

// CollectionArrowTile renders a router Link, so any screen with a resolved
// arrow needs a real router in scope -- mirrors the harness in arrow-card.test.tsx.
function renderScreen(namespace: string) {
	const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const rootRoute = createRootRoute({
		component: () => <CollectionDetailScreen namespace={namespace} />,
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

	return render(
		<QueryClientProvider client={client}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	);
}

describe('CollectionDetailScreen', () => {
	it('shows a loading state before the fetch resolves', async () => {
		vi.mocked(apiFetch).mockReturnValue(new Promise(() => {}));
		renderScreen('github.com/rabbyte/game-servers');
		expect(await screen.findByText(/loading/i)).toBeInTheDocument();
	});

	it('shows an error state immediately for an empty namespace, without ever fetching', async () => {
		vi.mocked(apiFetch).mockClear();
		renderScreen('');
		expect(await screen.findByText(/couldn't load this collection/i)).toBeInTheDocument();
		expect(apiFetch).not.toHaveBeenCalled();
	});

	it('shows the collection, its resolved arrows, and the unresolved count once fetched', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [
				{ namespace: 'github.com/rabbyte/minecraft@v1.21.4', resolved: true, name: 'Minecraft Server' },
				{ namespace: 'github.com/rabbyte/ark-survival@v3.1.0', resolved: false },
			],
		});

		renderScreen('github.com/rabbyte/game-servers');

		expect(await screen.findByText('Game Servers')).toBeInTheDocument();
		expect(screen.getAllByText('Minecraft Server').length).toBeGreaterThan(0);
		expect(screen.getByRole('button', { name: /1 unresolved/ })).toBeInTheDocument();
	});

	it('shows an error state on fetch failure', async () => {
		vi.mocked(apiFetch).mockRejectedValue(new Error('not found'));
		renderScreen('github.com/rabbyte/missing');
		expect(await screen.findByText(/couldn't load this collection/i)).toBeInTheDocument();
	});

	it('opens the unresolved dialog from the hero pill, with the version reattached to the route', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [{ namespace: 'github.com/rabbyte/ark-survival@v3.1.0', resolved: false }],
		});

		renderScreen('github.com/rabbyte/game-servers');

		const pill = await screen.findByRole('button', { name: /1 unresolved/ });
		fireEvent.click(pill);
		expect(await screen.findByText('github.com/rabbyte/ark-survival@v3.1.0')).toBeInTheDocument();
	});

	it('lists an unresolved route bare when it carries no version', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/game-servers',
			name: 'Game Servers',
			followed: true,
			arrows: [{ namespace: 'github.com/rabbyte/ark-survival', resolved: false }],
		});

		renderScreen('github.com/rabbyte/game-servers');

		const pill = await screen.findByRole('button', { name: /1 unresolved/ });
		fireEvent.click(pill);
		expect(await screen.findByText('github.com/rabbyte/ark-survival')).toBeInTheDocument();
	});

	it('omits the unresolved pill and dialog trigger when every arrow resolved', async () => {
		vi.mocked(apiFetch).mockResolvedValue({
			namespace: 'github.com/rabbyte/homelab',
			name: 'Homelab Essentials',
			followed: false,
			arrows: [{ namespace: 'github.com/rabbyte/caddy@v2.8.4', resolved: true, name: 'Caddy' }],
		});

		renderScreen('github.com/rabbyte/homelab');

		expect(await screen.findByText('Homelab Essentials')).toBeInTheDocument();
		expect(screen.queryByText(/unresolved/)).not.toBeInTheDocument();
	});
});
