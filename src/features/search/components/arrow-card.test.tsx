import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { ArrowCard } from './arrow-card';

const ENTRY: SearchEntry = {
	namespace: 'github.com/rabbyte/minecraft',
	name: 'Minecraft Server',
	description: 'Vanilla dedicated server.',
	tags: ['game'],
	icon: 'data:image/gif;base64,R0lGODlhAQABAAAAACw=',
	banner: 'data:image/gif;base64,R0lGODlhAQABAAAAACw=',
	versions: ['v1.21.4'],
	compatible_os: ['darwin/arm64'],
	provenance: 'installed',
	installed: true,
	known: true,
	stars: 12,
	source: 'github.com',
};

async function renderCard(entry: SearchEntry) {
	const rootRoute = createRootRoute({
		component: () => <ArrowCard entry={entry} />,
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

describe('ArrowCard', () => {
	it('names the arrow for assistive tech even though the name is visually hidden at rest', async () => {
		await renderCard(ENTRY);
		expect(screen.getByRole('link', { name: /Minecraft Server/ })).toBeInTheDocument();
	});

	it('links to the arrow detail route by bare namespace', async () => {
		// The splat segment carries slashes verbatim (matching ArrowRow's `_splat`
		// usage) rather than percent-encoding them -- it captures a raw multi-segment path.
		const { link } = await renderCard(ENTRY);
		expect(link).toHaveAttribute('href', '/arrow/github.com/rabbyte/minecraft');
	});

	it('shows the namespace under the name in the reveal strip', async () => {
		await renderCard(ENTRY);
		expect(screen.getByText('github.com/rabbyte/minecraft')).toBeInTheDocument();
	});

	it('renders the banner as a background rather than an img, so it can be lifted', async () => {
		const { container } = await renderCard(ENTRY);
		const banner = container.querySelector('[data-slot="card-banner"]');
		expect(banner).toHaveStyle({ backgroundImage: `url(${ENTRY.banner})` });
	});

	it('marks a result that is merely known, so the merge is visible to tests', async () => {
		const { link } = await renderCard({ ...ENTRY, installed: false, known: true, provenance: 'seen' });
		expect(link).toHaveAttribute('data-provenance', 'seen');
	});

	it('renders no provenance attribute when the server did not say', async () => {
		const { link } = await renderCard({ ...ENTRY, provenance: null });
		expect(link).not.toHaveAttribute('data-provenance');
	});

	it('reveals the name and namespace on keyboard focus, not only on mouse hover', async () => {
		const { container } = await renderCard(ENTRY);
		const info = container.querySelector('[data-slot="card-info"]');
		expect(info?.className).toMatch(/group-hover:opacity-100/);
		expect(info?.className).toMatch(/group-focus-visible:opacity-100/);
	});
});
