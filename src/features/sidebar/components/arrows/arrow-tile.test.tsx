import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { ArrowStatus } from '@/features/arrow-details/lib/status';

import { ArrowTile, type ArrowTileProps } from './arrow-tile';

/**
 * The router's initial match/render is asynchronous even with no loaders --
 * mirrors arrow-card.test.tsx's own harness, which awaits the link before
 * returning rather than asserting against a possibly-still-pending render.
 */
async function renderTile(props: Partial<ArrowTileProps> = {}) {
	const merged: ArrowTileProps = {
		to: '/arrow/$',
		namespace: 'github.com/rabbyte/minecraft',
		title: 'Minecraft Server',
		subtitle: 'Vanilla dedicated server.',
		icon: null,
		banner: null,
		metaText: 'v1.21.4',
		...props,
	};

	const rootRoute = createRootRoute({ component: () => <ArrowTile {...merged} /> });
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

	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute, collectionRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	const view = render(<RouterProvider router={router} />);
	const link = await screen.findByRole('link');
	return { ...view, router, link };
}

const READY: ArrowStatus = { labelKey: 'arrow.state.ready', iconKind: 'ready' };
const BUSY: ArrowStatus = { labelKey: 'arrow.state.installing', iconKind: 'busy' };
const ACTIVE: ArrowStatus = { labelKey: 'arrow.state.running', iconKind: 'active' };
const UP: ArrowStatus = { labelKey: 'arrow.state.outdated', iconKind: 'up' };
const PROBLEM: ArrowStatus = { labelKey: 'arrow.state.detached', iconKind: 'problem' };

describe('ArrowTile', () => {
	it('names the tile for assistive tech even though the name is visually hidden at rest', async () => {
		await renderTile();
		expect(screen.getByRole('link', { name: /Minecraft Server/ })).toBeInTheDocument();
	});

	it('links to /arrow/$ with the namespace as the splat param', async () => {
		const { link } = await renderTile({ to: '/arrow/$', namespace: 'github.com/rabbyte/minecraft' });
		expect(link).toHaveAttribute('href', '/arrow/github.com/rabbyte/minecraft');
	});

	it('links to /collection/$ with the namespace as the splat param', async () => {
		const { link } = await renderTile({ to: '/collection/$', namespace: 'guild/frosthold-pack' });
		expect(link).toHaveAttribute('href', '/collection/guild/frosthold-pack');
	});

	it('draws a monogram fallback banner when there is no real banner image', async () => {
		await renderTile({ title: 'Minecraft Server', banner: null });
		expect(screen.getByText('M')).toBeInTheDocument();
	});

	it('renders a real banner image instead of the drawn fallback when one is set', async () => {
		await renderTile({ banner: 'https://example.com/banner.png' });
		expect(screen.queryByText('M')).not.toBeInTheDocument();
	});

	it('overlays a drawn mark for a real icon on top of a drawn (bannerless) fallback', async () => {
		const { container } = await renderTile({ banner: null, icon: 'https://example.com/icon.png' });
		expect(container.querySelector('[data-slot="drawn-mark"]')).toBeInTheDocument();
	});

	it('draws no mark when there is neither a real icon nor a real banner', async () => {
		const { container } = await renderTile({ banner: null, icon: null });
		expect(container.querySelector('[data-slot="drawn-mark"]')).not.toBeInTheDocument();
	});

	it('shows the meta text in the hover-reveal strip', async () => {
		await renderTile({ metaText: '14 arrows' });
		expect(screen.getByText('14 arrows')).toBeInTheDocument();
	});

	it('renders no status badge when status is omitted', async () => {
		await renderTile({ status: null });
		expect(screen.queryByText('Ready')).not.toBeInTheDocument();
	});

	it.each([
		[BUSY, 'Installing…'],
		[ACTIVE, 'Running'],
		[UP, 'Update available'],
		[PROBLEM, 'Detached'],
		[READY, 'Ready'],
	])('renders the %o badge label', async (status, label) => {
		await renderTile({ status });
		expect(screen.getByText(label)).toBeInTheDocument();
	});

	it('only makes the badge interactive for a busy status with a resolve handler', async () => {
		await renderTile({ status: BUSY, onResolve: vi.fn() });
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
	});

	it('does not call onResolve if a non-resolvable badge is clicked anyway', async () => {
		const onResolve = vi.fn();
		await renderTile({ status: BUSY, onResolve });
		fireEvent.click(screen.getByText('Installing…'));
		expect(onResolve).not.toHaveBeenCalled();
	});

	it('does not make the badge interactive for a problem status without a resolve handler', async () => {
		await renderTile({ status: PROBLEM });
		expect(screen.queryByRole('button')).not.toBeInTheDocument();
	});

	it('resolves a detached arrow on click without navigating the tile', async () => {
		const onResolve = vi.fn();
		const { router } = await renderTile({ status: PROBLEM, onResolve });

		fireEvent.click(screen.getByRole('button', { name: 'Detached' }));

		expect(onResolve).toHaveBeenCalledTimes(1);
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(router.state.location.pathname).toBe('/');
	});

	it('resolves a detached arrow on Enter and Space, but not on other keys', async () => {
		const onResolve = vi.fn();
		await renderTile({ status: PROBLEM, onResolve });
		const badge = screen.getByRole('button', { name: 'Detached' });

		fireEvent.keyDown(badge, { key: 'a' });
		expect(onResolve).not.toHaveBeenCalled();

		fireEvent.keyDown(badge, { key: 'Enter' });
		fireEvent.keyDown(badge, { key: ' ' });
		expect(onResolve).toHaveBeenCalledTimes(2);
	});
});
