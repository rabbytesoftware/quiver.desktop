import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { TooltipProvider } from '@/components/ui/tooltip';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import type { ArrowEntry } from '@/domain/arrow';
import { SIDEBAR_DEFAULT, useShellStore, type SidebarSide } from '@/features/shell';
import { useArrowStore } from '@/lib/core-store';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';

import { Sidebar } from './components/sidebar';

const MINECRAFT = 'github.com/rabbyte/minecraft@v1.21.4';
const TERRARIA = 'github.com/rabbyte/terraria@v1.4.4';

function entry(namespace: string, name: string): ArrowEntry {
	return {
		namespace,
		name,
		description: '',
		tags: [],
		icon: null,
		banner: null,
		version: 'v1',
		state: 'absent',
		active_run: null,
		last_return: null,
	};
}

/**
 * The whole rail, on the five routes the app has.
 *
 * Hand-built rather than taken from `routeTree.gen`: the generated tree mounts
 * `__root.tsx`, which mounts an `AppShell` that mounts a rail of its own — and
 * every "exactly one thing is active" assertion below would then be counting
 * two rails' worth of links. The route PATHS still have to match the real ones
 * exactly, because those strings are what the rail's `<Link>`s resolve against.
 *
 * The rail goes in the ROOT component, which is where the shell puts it: above
 * the `<Outlet/>`, so nothing here re-renders on navigation. A rail mounted
 * under a leaf would re-render every time and a row that failed to pick up the
 * router's marking on its own would look like it worked.
 */
async function renderRail(at: string, side: SidebarSide = 'left') {
	useShellStore.setState({ sidebarSide: side, sidebarWidth: SIDEBAR_DEFAULT });

	const rootRoute = createRootRoute({
		component: () => (
			<TooltipProvider>
				<Sidebar />
				<Outlet />
			</TooltipProvider>
		),
	});
	const routes = [
		createRoute({ getParentRoute: () => rootRoute, path: '/', component: () => <div data-testid="page-home" /> }),
		createRoute({
			getParentRoute: () => rootRoute,
			path: '/remote',
			component: () => <div data-testid="page-remote" />,
		}),
		createRoute({
			getParentRoute: () => rootRoute,
			path: '/settings',
			component: () => <div data-testid="page-settings" />,
		}),
		createRoute({
			getParentRoute: () => rootRoute,
			path: '/search',
			component: () => <div data-testid="page-search" />,
		}),
		createRoute({
			getParentRoute: () => rootRoute,
			path: '/arrow/$',
			component: () => <div data-testid="page-arrow" />,
		}),
	];

	const router = createRouter({
		routeTree: rootRoute.addChildren(routes),
		history: createMemoryHistory({ initialEntries: [at] }),
	});

	const { container } = render(<RouterProvider router={router} />);
	await screen.findAllByRole('link');

	const rail = container.querySelector<HTMLElement>('[data-slot="sidebar"]');
	if (rail === null) throw new Error('the rail did not render');
	return { rail, router };
}

/**
 * Opens an arrow the way the rail does — through the router, with the namespace
 * as a param.
 *
 * NOT `initialEntries: ['/arrow/' + namespace]`. TanStack percent-encodes the
 * `@` when it builds a location, so `/arrow/github.com/rabbyte/minecraft@v1`
 * written by hand is a different string from the `%40` href on the row: no link
 * matches, no row is marked, and `activeNames` comes back empty. The trap is
 * that the invariant below reads that as a rail with nothing wrongly lit and
 * PASSES — for a route that never loaded.
 */
async function openArrow(router: Awaited<ReturnType<typeof renderRail>>['router'], namespace: string) {
	await act(async () => {
		await router.navigate({ to: '/arrow/$', params: { _splat: namespace } });
	});
	await screen.findByTestId('page-arrow');
}

/**
 * Everything in the rail the router has marked, named.
 *
 * Names rather than a count so a failure says WHICH row lit up — the prefix
 * trap fails as `['Home', 'Minecraft']`, and a count alone would not say which
 * of the two was the intruder. A nav segment carries its name as `aria-label`;
 * an arrow row's is the slot inside it.
 */
function activeNames(rail: HTMLElement): string[] {
	return Array.from(rail.querySelectorAll('[data-status="active"]')).map(
		(row) => row.getAttribute('aria-label') ?? row.querySelector('[data-slot="arrow-name"]')?.textContent ?? ''
	);
}

/** The rail's top bar — the row the history buttons and the reserve share. */
function topBar(): HTMLElement {
	const back = screen.getByRole('button', { name: 'Go back' });
	const bar = back.parentElement?.parentElement;
	if (!bar) throw new Error('the history buttons are not in a top bar');
	return bar;
}

function arrowRow(name: string): HTMLElement {
	const row = screen.getByText(name, { selector: '[data-slot="arrow-name"]' }).closest('a');
	if (row === null) throw new Error(`no arrow row for ${name}`);
	return row;
}

beforeEach(() => {
	useArrowStore.getState().reset();
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
	localStorage.removeItem(LOCALE_STORAGE_KEY);
	useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
});

afterEach(() => {
	// Module state otherwise: a case that pretends to be macOS answers for every
	// case after it, and the reserve appears in rails that never asked for one.
	restoreUserAgent();
});

/**
 * THE invariant, and the reason there is no selection store to test instead
 * (spec §5.1). Home, Remote, Settings and the arrow rows are one navigation, so
 * exactly one thing in the rail is active — and because every row is a `<Link>`
 * and the router does the marking, that is something the rail cannot violate
 * rather than something a reducer has to get right.
 *
 * Scoped to the rail element, not the document: the assertion is about the
 * WHOLE rail at once, which is what a per-component test cannot say. A nav
 * segment lit on top of an arrow row passes both `primary-nav.test.tsx` and
 * `arrow-list.test.tsx` and fails only here.
 */
describe('the rail has exactly one active row', () => {
	beforeEach(() => {
		useArrowStore.setState({
			arrows: new Map([
				[MINECRAFT, entry(MINECRAFT, 'Minecraft')],
				[TERRARIA, entry(TERRARIA, 'Terraria')],
			]),
		});
	});

	it('lights Home, and nothing else, at /', async () => {
		const { rail } = await renderRail('/');
		expect(activeNames(rail)).toEqual(['Home']);
	});

	it('lights Remote, and nothing else, at /remote', async () => {
		const { rail } = await renderRail('/remote');
		expect(activeNames(rail)).toEqual(['Remote']);
	});

	it('lights Settings, and nothing else, at /settings', async () => {
		const { rail } = await renderRail('/settings');
		expect(activeNames(rail)).toEqual(['Settings']);
	});

	it('lights the open arrow, and nothing in the nav, at its route', async () => {
		const { rail, router } = await renderRail('/');
		await openArrow(router, MINECRAFT);
		expect(activeNames(rail)).toEqual(['Minecraft']);
	});

	/**
	 * Nothing in the rail owns `/search`; the field's own inversion is what is
	 * lit while searching (spec §5.1). Zero is the correct answer here and the
	 * one a broken `@` in `openArrow` also produces, which is why that helper
	 * navigates through the router.
	 */
	it('lights nothing anywhere at /search', async () => {
		const { rail } = await renderRail('/search');
		expect(activeNames(rail)).toEqual([]);
	});
});

describe('Sidebar', () => {
	it('spans both grid rows so no blank band sits above the rail', async () => {
		// `grid-row: 1 / 3` in every combination (spec §1.1). Without the span
		// the rail starts below the chrome row and its own first row — the one
		// holding the reserve and the history buttons — has nowhere to be.
		const { rail } = await renderRail('/');
		expect(rail.className).toContain('row-span-2');
	});

	/**
	 * `ResizeHandle` is `absolute inset-y-0`. With no positioned ancestor it
	 * resolves against whatever further up happens to be positioned, so the
	 * four-pixel strip lands somewhere across the window and the rail cannot be
	 * dragged at all — while looking entirely correct.
	 */
	it('is the positioned ancestor the resize handle anchors to', async () => {
		const { rail } = await renderRail('/');
		const handle = screen.getByRole('separator', { name: 'Resize sidebar' });

		expect(rail.className).toContain('relative');
		expect(handle.className).toContain('absolute');
		expect(handle.closest('.relative')).toBe(rail);
	});

	it.each([
		['left', 'border-r', 'border-l'],
		['right', 'border-l', 'border-r'],
	] as const)('puts the divider on the content-facing edge with the rail on the %s', async (side, faces, away) => {
		// The divider rides the RAIL (spec §1.3): the content column occupies
		// row 2 only, so the same border over there stops short of the top and
		// leaves the chrome row undivided.
		const { rail } = await renderRail('/', side);
		expect(rail.className).toContain(faces);
		expect(rail.className).not.toContain(away);
	});

	/**
	 * The rail's own top bar and nav are fixed heads of the column. Scroll the
	 * rail instead and the back button — the one control that undoes a wrong
	 * click — leaves the screen as soon as the library is longer than the
	 * window.
	 */
	it('scrolls the arrow list and nothing above it', async () => {
		useArrowStore.setState({ arrows: new Map([[MINECRAFT, entry(MINECRAFT, 'Minecraft')]]) });
		const { rail } = await renderRail('/');

		const scroller = rail.querySelector('[data-slot="scroll-area"]');
		expect(scroller).toContainElement(screen.getByRole('navigation', { name: 'Arrows' }));
		expect(scroller).not.toContainElement(topBar());
		expect(scroller).not.toContainElement(screen.getByRole('link', { name: 'Home' }));
	});
});

/**
 * Spec §5.8: the window's edge belongs to the OS, the interior belongs to the
 * app. The two never share an edge, or macOS paints its three lights over back
 * and forward and both are unreachable for the window's life.
 *
 * Asserted as DOM order within the top bar, which is the only thing jsdom can
 * see — it has no layout engine, so "on the right" is not a question it can
 * answer.
 */
describe('RailTopBar', () => {
	it('faces the history buttons at the content with the rail on the left', async () => {
		await renderRail('/', 'left');
		expect(topBar().lastElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('faces them at the content with the rail on the right', async () => {
		await renderRail('/', 'right');
		expect(topBar().firstElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('gives the window edge to the macOS reserve and the interior to history', async () => {
		runningOn(USER_AGENTS.macos);
		await renderRail('/', 'left');

		// Reserve first, history last: the lights are on the window's left edge
		// and the content column is to the rail's right.
		expect(topBar().firstElementChild).toHaveAttribute('data-tauri-drag-region');
		expect(topBar().lastElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('leaves the reserve to the chrome row on macOS with the rail on the right', async () => {
		// The one combination where the lights are not on the rail's edge at all
		// (spec §4.5). Held here as well, the window opens 64px twice — once in
		// each column — for one set of buttons.
		runningOn(USER_AGENTS.macos);
		await renderRail('/', 'right');
		expect(topBar().querySelector('[data-tauri-drag-region]')).toBeNull();
	});

	it.each([
		['Linux', USER_AGENTS.linux],
		['Windows', USER_AGENTS.windows],
	])('reserves nothing on %s, where the OS draws its own title bar', async (_platform, userAgent) => {
		runningOn(userAgent);
		await renderRail('/', 'left');

		expect(topBar().querySelector('[data-tauri-drag-region]')).toBeNull();
		// Still on the interior edge with nothing beside it — `justify-between`
		// would have collapsed it onto the window's edge here.
		expect(topBar().lastElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('is one row tall, from the token rather than from pixels', async () => {
		// `h-[34px]` looks right today and stops tracking `--row` the moment the
		// scale moves, with no layout assertion possible in jsdom to notice.
		await renderRail('/');
		expect(topBar().className).toContain('h-(--row)');
	});
});

/**
 * Spec §5.9. Clicking the row you are already on must not push, or the entry
 * behind you is the page you are looking at: Back re-renders the same screen,
 * nothing moves, and there is nothing on screen saying how many identical
 * entries are stacked up.
 *
 * THE TRAP IN TESTING THIS: TanStack already declines to push when the href it
 * builds is byte-identical to the current one, so `/` → `/` proves nothing and
 * passes with the guard deleted. The reachable case is a row that is ACTIVE at
 * a location its own href does not equal — which is the normal state of the
 * rail, because a rail link carries no search and no sub-path, and the router
 * marks a link active on a path prefix with a subset of its search.
 *
 * `/settings?tab=developer` is that case in this app today: the Settings panel
 * writes the tab into the URL, the rail's segment stays lit, and the panel
 * remembers its tab in a store — so a push lands you on a page that looks
 * identical, and Back returns you to one that looks identical too.
 */
describe('re-selecting the active row', () => {
	beforeEach(() => {
		useArrowStore.setState({
			arrows: new Map([
				[MINECRAFT, entry(MINECRAFT, 'Minecraft')],
				[TERRARIA, entry(TERRARIA, 'Terraria')],
			]),
		});
	});

	it('pushes nothing when the active nav segment is clicked again', async () => {
		const user = userEvent.setup();
		const { rail, router } = await renderRail('/settings?tab=developer');
		expect(activeNames(rail)).toEqual(['Settings']);
		const before = router.history.length;

		await user.click(screen.getByRole('link', { name: 'Settings' }));

		expect(router.history.length).toBe(before);
		// Not merely "no entry" — the click is a no-op, so the tab the panel is
		// showing is still in the URL that opened it.
		expect(router.state.location.searchStr).toBe('?tab=developer');
	});

	it('still pushes when a different destination is clicked', async () => {
		const user = userEvent.setup();
		const { router } = await renderRail('/settings?tab=developer');

		await user.click(screen.getByRole('link', { name: 'Home' }));

		expect(await screen.findByTestId('page-home')).toBeInTheDocument();
		expect(router.history.canGoBack()).toBe(true);
	});

	/**
	 * The same shape one level down: `/arrow/$` is a splat, so the row for an
	 * arrow is marked active anywhere beneath its own path — and its href is
	 * then not where you are. Without the guard this is a push per click onto a
	 * row that was already lit.
	 */
	it('pushes nothing when the active arrow row is clicked again', async () => {
		const user = userEvent.setup();
		const { rail, router } = await renderRail('/');
		await openArrow(router, `${MINECRAFT}/logs`);
		expect(activeNames(rail)).toEqual(['Minecraft']);
		const before = router.history.length;

		await user.click(arrowRow('Minecraft'));

		expect(router.history.length).toBe(before);
	});

	it('still pushes when another arrow is clicked', async () => {
		const user = userEvent.setup();
		const { rail, router } = await renderRail('/');
		await openArrow(router, MINECRAFT);
		const before = router.history.length;

		await user.click(arrowRow('Terraria'));

		expect(router.history.length).toBe(before + 1);
		expect(activeNames(rail)).toEqual(['Terraria']);
	});
});
