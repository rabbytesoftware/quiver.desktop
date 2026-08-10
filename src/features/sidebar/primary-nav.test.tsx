import { GearIcon, HardDrivesIcon, HouseIcon } from '@phosphor-icons/react';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { TooltipProvider } from '@/components/ui/tooltip';

import { PrimaryNav } from './components/primary-nav';

async function renderNav(at: string) {
	const rootRoute = createRootRoute({
		component: () => (
			<TooltipProvider>
				<PrimaryNav />
				<Outlet />
			</TooltipProvider>
		),
	});
	const indexRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home-page" />,
	});
	const remoteRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/remote',
		component: () => <div data-testid="remote-page" />,
	});
	const settingsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/settings',
		component: () => <div data-testid="settings-page" />,
	});
	const searchRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/search',
		component: () => <div data-testid="search-page" />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, remoteRoute, settingsRoute, searchRoute, arrowRoute]),
		history: createMemoryHistory({ initialEntries: [at] }),
	});

	render(<RouterProvider router={router} />);
	await screen.findAllByRole('tab');
	return router;
}

function activeLabels(): string[] {
	return screen
		.getAllByRole('tab')
		.filter((link) => link.getAttribute('data-status') === 'active')
		.map((link) => link.getAttribute('aria-label') ?? '');
}

describe('PrimaryNav', () => {
	it('lights Home, and only Home, at the index route', async () => {
		await renderNav('/');
		expect(activeLabels()).toEqual(['Home']);
	});

	it('lights Remote, and only Remote, at /remote', async () => {
		await renderNav('/remote');
		expect(activeLabels()).toEqual(['Remote']);
	});

	it('lights Settings, and only Settings, at /settings', async () => {
		await renderNav('/settings');
		expect(activeLabels()).toEqual(['Settings']);
	});

	it('does not light Home at /remote', async () => {
		await renderNav('/remote');
		expect(screen.getByRole('tab', { name: 'Home' })).not.toHaveAttribute('data-status');
	});

	it('lights nothing at /search', async () => {
		await renderNav('/search');
		expect(activeLabels()).toEqual([]);
	});

	it('lights nothing at an arrow route', async () => {
		await renderNav('/arrow/github.com/x/y@v1');
		expect(activeLabels()).toEqual([]);
	});

	it('moves the active segment when another is clicked', async () => {
		const user = userEvent.setup();
		await renderNav('/');

		await user.click(screen.getByRole('tab', { name: 'Settings' }));

		expect(await screen.findByTestId('settings-page')).toBeInTheDocument();
		expect(activeLabels()).toEqual(['Settings']);
	});

	it('reveals labels on crowbar’s ladder — active at 280px, all at 420px', async () => {
		await renderNav('/remote');

		const labelOf = (name: string) =>
			[...screen.getByRole('tab', { name }).querySelectorAll('span')].find((span) => span.textContent === name);

		const active = labelOf('Remote');
		expect(active?.className).toContain('hidden');
		expect(active?.className).toContain('@[280px]:inline');
		expect(active?.className).toContain('@[420px]:inline');

		const collapsed = labelOf('Home');
		expect(collapsed?.className).toContain('hidden');
		expect(collapsed?.className).not.toContain('@[280px]:inline');
		expect(collapsed?.className).toContain('@[420px]:inline');
	});

	it('gives the collapsed segments a tooltip and the active one none', async () => {
		await renderNav('/remote');

		expect(screen.getByRole('tab', { name: 'Remote' }).querySelector('[data-slot=tooltip-trigger]')).toBeNull();
		expect(screen.getByRole('tab', { name: 'Home' }).querySelector('[data-slot=tooltip-trigger]')).not.toBeNull();
		expect(
			screen.getByRole('tab', { name: 'Settings' }).querySelector('[data-slot=tooltip-trigger]')
		).not.toBeNull();
	});

	it('keeps every segment addressable as a tab, tooltip or not', async () => {
		await renderNav('/remote');
		expect(document.querySelectorAll('[data-slot="tabs-tab"]')).toHaveLength(3);
	});

	it('names every segment whether or not it is showing its label', async () => {
		await renderNav('/search');
		for (const name of ['Home', 'Remote', 'Settings']) {
			expect(screen.getByRole('tab', { name })).toHaveAttribute('aria-label', name);
		}
	});

	it('shares the track equally, with no segment growing when active', async () => {
		await renderNav('/');
		for (const name of ['Home', 'Remote', 'Settings']) {
			const tab = screen.getByRole('tab', { name });
			expect(tab.className).toContain('flex-1');
			expect(tab.className).not.toContain('max-w-[54%]');
		}
	});

	it('inverts the indicator rather than raising it', async () => {
		await renderNav('/');

		const indicator = document.querySelector('[data-slot="tab-indicator"]');
		expect(indicator?.className).toContain('bg-foreground');
		expect(indicator?.className).not.toContain('bg-background');
	});

	it('drops the indicator entirely when no destination is active', async () => {
		await renderNav('/search');

		expect(document.querySelector('[data-slot="tab-indicator"]')).toBeNull();
		for (const name of ['Home', 'Remote', 'Settings']) {
			expect(screen.getByRole('tab', { name })).toHaveAttribute('aria-selected', 'false');
		}
	});

	it('fills the active glyph and leaves the others regular', async () => {
		await renderNav('/');

		for (const [name, Icon, weight] of [
			['Home', HouseIcon, 'fill'],
			['Remote', HardDrivesIcon, 'regular'],
			['Settings', GearIcon, 'regular'],
		] as const) {
			const glyph = screen.getByRole('tab', { name }).querySelector('svg');
			const { container } = render(<Icon weight={weight} />);

			expect(glyph).toHaveAttribute('fill', 'currentColor');
			expect(glyph?.innerHTML).toBe(container.querySelector('svg')?.innerHTML);
		}
	});
});
