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

import { CollectionArrowGrid } from './collection-arrow-grid';

const ARROWS: CollectionArrow[] = [
	{ namespace: 'github.com/rabbyte/minecraft', resolved: true, name: 'Minecraft Server' },
	{ namespace: 'github.com/rabbyte/ark-survival', resolved: false },
];

// CollectionArrowTile renders a router Link, so any grid with a resolved
// arrow needs a real router in scope -- mirrors the harness in arrow-card.test.tsx.
async function renderGrid(arrows: CollectionArrow[]) {
	const rootRoute = createRootRoute({
		component: () => <CollectionArrowGrid arrows={arrows} />,
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
	await screen.findAllByRole('link');
	return view;
}

describe('CollectionArrowGrid', () => {
	it('renders only resolved arrows', async () => {
		await renderGrid(ARROWS);
		expect(screen.getAllByText('Minecraft Server').length).toBeGreaterThan(0);
		expect(screen.queryByText('ark-survival')).not.toBeInTheDocument();
	});

	it('renders nothing when every arrow is unresolved', () => {
		const { container } = render(
			<CollectionArrowGrid arrows={[{ namespace: 'github.com/rabbyte/ark-survival', resolved: false }]} />
		);
		expect(container.querySelector('.collection-member-cell')).not.toBeInTheDocument();
	});

	it('keys resolved arrows by their full route so two versions of the same namespace do not collide', async () => {
		const arrows: CollectionArrow[] = [
			{ namespace: 'github.com/rabbyte/tool', version: 'v1', resolved: true, name: 'Tool v1' },
			{ namespace: 'github.com/rabbyte/tool', version: 'v2', resolved: true, name: 'Tool v2' },
		];
		await renderGrid(arrows);
		expect(screen.getAllByRole('link')).toHaveLength(2);
		expect(screen.getAllByText('Tool v1').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Tool v2').length).toBeGreaterThan(0);
	});
});
