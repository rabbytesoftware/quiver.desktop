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

/**
 * The five real routes, hand-built rather than imported from `routeTree.gen`:
 * `__root.tsx` mounts the whole app, so importing the generated tree would put a
 * second rail on screen and leave every `getAllByRole('link')` below counting
 * somebody else's links.
 *
 * The nav goes in the ROOT component, which is where the rail puts it — under a
 * leaf it would re-render on every navigation and a stale active state would
 * look reactive.
 */
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
	await screen.findAllByRole('link');
	return router;
}

/**
 * Every segment the router has marked, named. An array rather than a count so a
 * failure says *which* segment lit up instead of only how many did — the
 * prefix-match trap fails as `['Home', 'Remote']`, and the count alone would not
 * say which one was the intruder.
 */
function activeLabels(): string[] {
	return screen
		.getAllByRole('link')
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

	/**
	 * The prefix-match trap, and the reason `activeOptions={{ exact: true }}` is
	 * on the Home segment. TanStack matches by prefix, so `/` is a prefix of
	 * every route in the app: without it Home stays lit on top of whatever else
	 * is lit and "exactly one thing is active" is broken on the first click.
	 */
	it('does not light Home at /remote', async () => {
		await renderNav('/remote');
		expect(screen.getByRole('link', { name: 'Home' })).not.toHaveAttribute('data-status');
	});

	// Nothing in the rail owns /search — the field's own inversion is what is lit
	// while searching (spec §5.1).
	it('lights nothing at /search', async () => {
		await renderNav('/search');
		expect(activeLabels()).toEqual([]);
	});

	// An open arrow collapses all three segments, which needs no code: no nav
	// link matches, so all three hit the collapsed rule (spec §1.2).
	it('lights nothing at an arrow route', async () => {
		await renderNav('/arrow/github.com/x/y@v1');
		expect(activeLabels()).toEqual([]);
	});

	it('moves the active segment when another is clicked', async () => {
		const user = userEvent.setup();
		await renderNav('/');

		await user.click(screen.getByRole('link', { name: 'Settings' }));

		expect(await screen.findByTestId('settings-page')).toBeInTheDocument();
		expect(activeLabels()).toEqual(['Settings']);
	});

	// The label is what makes the wide slot worth 54% of the rail; a collapsed
	// segment that rendered its text too would just clip it.
	/**
	 * Every segment carries its label in the DOM at every width; only the CLASSES
	 * differ, because the ladder is a container query and jsdom has no layout to
	 * resolve one against. So the assertion is on the opt-in, not on the text:
	 * hidden by default, the active tab appearing at 280px and all three at 420.
	 * Those thresholds are crowbar's, copied rather than chosen — at Quiver's
	 * 246px rail the container is 230px, so the bar is icons only until it is
	 * dragged wider.
	 */
	it('reveals labels on crowbar’s ladder — active at 280px, all at 420px', async () => {
		await renderNav('/remote');

		const labelOf = (name: string) =>
			[...screen.getByRole('link', { name }).querySelectorAll('span')].find((span) => span.textContent === name);

		const active = labelOf('Remote');
		expect(active?.className).toContain('hidden');
		expect(active?.className).toContain('@[280px]:inline');
		expect(active?.className).toContain('@[420px]:inline');

		const collapsed = labelOf('Home');
		expect(collapsed?.className).toContain('hidden');
		// The inactive segments wait for the wide threshold and skip the middle one.
		expect(collapsed?.className).not.toContain('@[280px]:inline');
		expect(collapsed?.className).toContain('@[420px]:inline');
	});

	/**
	 * A tooltip on the active segment would repeat the label already on screen.
	 * Asserting the trigger rather than hovering because Base UI mounts the popup
	 * in a portal on a delay, and the question here is which segments have one at
	 * all — not what the popup looks like when it opens.
	 */
	it('gives the collapsed segments a tooltip and the active one none', async () => {
		await renderNav('/remote');

		expect(screen.getByRole('link', { name: 'Remote' }).querySelector('[data-slot=tooltip-trigger]')).toBeNull();
		expect(screen.getByRole('link', { name: 'Home' }).querySelector('[data-slot=tooltip-trigger]')).not.toBeNull();
		expect(
			screen.getByRole('link', { name: 'Settings' }).querySelector('[data-slot=tooltip-trigger]')
		).not.toBeNull();
	});

	/**
	 * Names the collapsed, icon-only segments. Without it a screen reader reads
	 * the role three times over, and the tooltip is no help: it hangs off a span
	 * inside the anchor, so its `aria-describedby` never reaches it.
	 */
	it('names every segment whether or not it is showing its label', async () => {
		await renderNav('/search');
		for (const name of ['Home', 'Remote', 'Settings']) {
			expect(screen.getByRole('link', { name })).toHaveAttribute('aria-label', name);
		}
	});

	/**
	 * Equal thirds, and the active one does NOT grow. The previous design gave it
	 * 54% of the rail; a segmented control marks itself by raising one of three
	 * fixed positions, and a segment that resized its neighbours as the indicator
	 * slid would read as three controls rather than one.
	 */
	it('shares the track equally, with no segment growing when active', async () => {
		await renderNav('/');
		for (const name of ['Home', 'Remote', 'Settings']) {
			const tab = screen.getByRole('link', { name });
			expect(tab.className).toContain('flex-1');
			expect(tab.className).not.toContain('max-w-[54%]');
		}
	});

	/**
	 * Weight follows state, which is crowbar's rule: `fill` on the active glyph,
	 * `regular` on the rest. Invisible to every other assertion here — drop the
	 * prop and the nav still renders, still lights up, still sizes itself — so
	 * only the path data can catch it, and comparing against a reference render
	 * says so without a geometry literal in the test.
	 *
	 * Both branches of the segment are covered: Home is active at `/` and renders
	 * its icon bare, the other two render theirs inside a tooltip trigger.
	 */
	it('fills the active glyph and leaves the others regular', async () => {
		await renderNav('/');

		for (const [name, Icon, weight] of [
			['Home', HouseIcon, 'fill'],
			['Remote', HardDrivesIcon, 'regular'],
			['Settings', GearIcon, 'regular'],
		] as const) {
			const glyph = screen.getByRole('link', { name }).querySelector('svg');
			const { container } = render(<Icon weight={weight} />);

			expect(glyph).toHaveAttribute('fill', 'currentColor');
			expect(glyph?.innerHTML).toBe(container.querySelector('svg')?.innerHTML);
		}
	});
});
