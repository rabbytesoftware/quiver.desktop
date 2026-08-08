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
	it('shows the label on the active segment and on no other', async () => {
		await renderNav('/remote');

		expect(screen.getByRole('link', { name: 'Remote' })).toHaveTextContent('Remote');
		expect(screen.getByRole('link', { name: 'Home' })).toHaveTextContent('');
		expect(screen.getByRole('link', { name: 'Settings' })).toHaveTextContent('');
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
	 * "link" three times over, and the tooltip is no help: it hangs off a span
	 * inside the anchor, so its `aria-describedby` never reaches the link.
	 */
	it('names every segment whether or not it is showing its label', async () => {
		await renderNav('/search');
		for (const name of ['Home', 'Remote', 'Settings']) {
			expect(screen.getByRole('link', { name })).toHaveAttribute('aria-label', name);
		}
	});

	/**
	 * The failure this catches is silent: `min-w-[34px]` and `size-5` both LOOK
	 * right today and stop tracking `--row` and `--icon` the moment either moves,
	 * with no layout assertion possible in jsdom to notice.
	 */
	it('sizes itself from the geometry tokens rather than from pixels', async () => {
		await renderNav('/');
		for (const name of ['Home', 'Remote', 'Settings']) {
			const link = screen.getByRole('link', { name });
			expect(link.className).toContain('h-(--row)');
			expect(link.className).toContain('not-data-[status=active]:min-w-(--row)');
			expect(link.querySelector('svg')?.getAttribute('class')).toContain('size-(--icon-nav)');
		}
	});

	// 54% of the rail, the proportion design.pen gives the active segment (112 of
	// 208). Uncapped it eats the whole rail at SIDEBAR_MAX.
	it('caps the active segment at 54% of the rail', async () => {
		await renderNav('/');
		expect(screen.getByRole('link', { name: 'Home' }).className).toContain('data-[status=active]:max-w-[54%]');
	});

	/**
	 * design.pen's own 610 and -0.1px, and the reason the weight is scoped to the
	 * active segment: it is the only one that renders text. Nothing here has a
	 * layout consequence, so the only way this drifts back to the browser default
	 * is silently.
	 */
	it('sets the active segment in design.pen weight and tracking', async () => {
		await renderNav('/');
		const home = screen.getByRole('link', { name: 'Home' });

		expect(home.className).toContain('data-[status=active]:font-[610]');
		expect(home.className).toContain('tracking-[-0.1px]');
	});

	/**
	 * Phosphor's weight is invisible to every other assertion here: delete
	 * `weight={WEIGHT}` and the nav still renders, still lights up, still sizes
	 * itself from the token, and all three glyphs quietly thin to the `regular`
	 * default — 0.88px of stroke at 14px where design.pen draws 1.23px. Only the
	 * path data changes, so only the path data can catch it, and comparing
	 * against a reference render says it without a geometry literal in the test.
	 *
	 * Both branches of the segment are covered by the one loop: Home is active at
	 * `/` and renders its icon bare, the other two render theirs inside a tooltip
	 * trigger.
	 *
	 * `fill` belongs here for the same reason. It is Phosphor's default rather
	 * than something this code sets, and it is what carries the glyph from
	 * `text-sidebar-primary-foreground` when the active segment inverts — pass a
	 * `color` and the icon stays dark on the dark fill.
	 */
	it('draws all three segments at one Phosphor weight, in the inherited colour', async () => {
		await renderNav('/');

		for (const [name, Icon] of [
			['Home', HouseIcon],
			['Remote', HardDrivesIcon],
			['Settings', GearIcon],
		] as const) {
			const glyph = screen.getByRole('link', { name }).querySelector('svg');
			const { container } = render(<Icon weight="bold" />);

			expect(glyph).toHaveAttribute('fill', 'currentColor');
			expect(glyph?.innerHTML).toBe(container.querySelector('svg')?.innerHTML);
		}
	});
});
