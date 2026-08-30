import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import type { CollectionArrow } from '@/domain/collection';

import { CollectionArrowTile } from './collection-arrow-tile';

// This renders a router Link (it navigates to /arrow/$ like the real
// ArrowCard it mirrors), so it needs a real router in scope -- mirrors the
// harness in arrow-card.test.tsx.
async function renderTile(arrow: CollectionArrow) {
	const rootRoute = createRootRoute({
		component: () => <CollectionArrowTile arrow={arrow} />,
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

	const view = render(<RouterProvider router={router} />);
	const link = await screen.findByRole('link');
	return { ...view, link };
}

describe('CollectionArrowTile', () => {
	it('renders the arrow name (in both the drawn-art overlay and the caption) and its bare namespace as the caption subtitle', async () => {
		await renderTile({
			namespace: 'github.com/rabbyte/minecraft',
			version: 'v1.21.4',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(screen.getAllByText('Minecraft Server')).toHaveLength(2);
		expect(screen.getByText('github.com/rabbyte/minecraft')).toBeInTheDocument();
	});

	it('shows the version in the info strip', async () => {
		await renderTile({
			namespace: 'github.com/rabbyte/minecraft',
			version: 'v1.21.4',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(screen.getByText('v1.21.4')).toBeInTheDocument();
	});

	it('omits the version text when there is none', async () => {
		await renderTile({ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' });
		expect(screen.queryByText(/^v\d/)).not.toBeInTheDocument();
	});

	it('falls back to the namespace tail when name is somehow missing', async () => {
		await renderTile({ namespace: 'github.com/rabbyte/minecraft', resolved: true });
		expect(screen.getAllByText('minecraft').length).toBeGreaterThan(0);
	});

	it('falls back to the whole namespace when its tail segment is empty', async () => {
		await renderTile({ namespace: 'github.com/rabbyte/', resolved: true });
		expect(screen.getAllByText('github.com/rabbyte/').length).toBeGreaterThan(0);
	});

	it('falls back to the first path segment as owner when the namespace has two segments or fewer', async () => {
		await renderTile({ namespace: 'standalone-repo', resolved: true, name: 'Standalone' });
		expect(screen.getAllByText('standalone-repo').length).toBeGreaterThan(0);
	});

	it('renders the ghost initial from the display name', async () => {
		await renderTile({ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' });
		expect(screen.getByText('M')).toBeInTheDocument();
	});

	it('renders the real data-slot markers card.css hooks its hover transform to', async () => {
		const { container } = await renderTile({
			namespace: 'github.com/rabbyte/minecraft',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(container.querySelector('[data-slot="arrow-card"]')).toBeInTheDocument();
		expect(container.querySelector('[data-slot="card-banner"]')).toBeInTheDocument();
	});

	it('links to the arrow detail route, reattaching the version quiver.core resolved it from', async () => {
		// The router percent-encodes `@` in a splat segment (unlike `/`, which
		// passes through verbatim) -- TanStack Router decodes it back to `@` on
		// the way in, so `_splat` on the destination route still reads the exact
		// route that was asked for.
		const { link } = await renderTile({
			namespace: 'github.com/rabbyte/minecraft',
			version: 'v1.21.4',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(link).toHaveAttribute('href', '/arrow/github.com/rabbyte/minecraft%40v1.21.4');
	});

	it('links to the bare namespace when there is no version', async () => {
		const { link } = await renderTile({
			namespace: 'github.com/rabbyte/minecraft',
			resolved: true,
			name: 'Minecraft Server',
		});
		expect(link).toHaveAttribute('href', '/arrow/github.com/rabbyte/minecraft');
	});
});
