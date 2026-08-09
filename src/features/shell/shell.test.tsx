import type { JSX } from 'react';

import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * `__root.tsx` mounts the real devtools panel in a dev build, and vitest is one.
 * Left alone it fires an async `import()` that resolves after the test that
 * triggered it has torn its DOM down, so the failure lands on whichever case
 * runs next.
 */
vi.mock('@tanstack/react-router-devtools', () => ({ TanStackRouterDevtools: () => null }));

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import { disposeMock, installMock } from '@/lib/mock';
import { resetBackend } from '@/lib/transport/backend';
import { Route as appRoot } from '@/routes/__root';

import { AppShell } from './components/app-shell';
import { WindowControls } from './components/window-controls';
import { ROW_H, windowControls, type SidebarSide } from './geometry';
import { SIDEBAR_DEFAULT, useShellStore } from './store';
import baseConfig from '../../../src-tauri/tauri.conf.json';
import macosConfig from '../../../src-tauri/tauri.macos.conf.json';

/** The three macOS-only window settings. Named once, asserted from both sides. */
const MACOS_ONLY_WINDOW_KEYS = ['titleBarStyle', 'hiddenTitle', 'trafficLightPosition'];

const baseWindow = baseConfig.app.windows[0] as Record<string, unknown>;
const macosWindow = macosConfig.app.windows[0];

/**
 * Every `tauri.<platform>.conf.json` that exists, keyed by the path written
 * below. Discovered rather than imported one by one, so a new platform overlay
 * shows up here without anyone remembering to add it — which is the point: an
 * overlay nobody notices is how Windows lost its window controls.
 *
 * `import.meta.glob` rather than `readdirSync`, because this project carries no
 * `@types/node` and `node:fs` therefore does not type-check. Vite resolves the
 * pattern at transform time, so the set is as fresh as the run. The pattern
 * cannot match `tauri.conf.json` itself: it requires a second `.` before
 * `conf`.
 */
const PLATFORM_OVERLAYS = import.meta.glob<{ app: { windows: Record<string, unknown>[] } }>(
	'../../../src-tauri/tauri.*.conf.json',
	{ eager: true, import: 'default' }
);

/**
 * The window settings a given platform actually starts with.
 *
 * RFC 7396 again: an overlay's `app.windows` REPLACES the shared array whole,
 * so a platform that has an overlay gets that overlay's window and NOTHING of
 * the shared one, while a platform without one gets the shared window verbatim.
 */
function effectiveWindow(platform: string): Record<string, unknown> {
	return PLATFORM_OVERLAYS[`../../../src-tauri/tauri.${platform}.conf.json`]?.app.windows[0] ?? baseWindow;
}

/** The visible circle, in CSS px. Three of them plus their gaps span ~64. */
const TRAFFIC_LIGHT_DIAMETER_PX = 12;

/**
 * The close button's FRAME height, which is larger than the 12pt circle drawn
 * inside it — the frame is the hit target.
 *
 * Measured, not documented: AppKit does not publish it, and it is the only
 * unknown in tao's placement. Read off the running window at a known `y` and
 * back-solved through the formula below.
 */
const TRAFFIC_LIGHT_FRAME_PX = 14;

/**
 * The reserve, rendered as the platform in `userAgent` would see it.
 *
 * Queried by the drag-region attribute rather than by tag: what the element is
 * called is nobody's business, and what it is FOR is the attribute.
 */
function renderControls(userAgent: string): HTMLElement | null {
	runningOn(userAgent);
	return render(<WindowControls />).container.querySelector('[data-slot="window-controls"]');
}

/**
 * A memory router around `component`, hand-built rather than taken from
 * `routeTree.gen`: the generated tree mounts `__root.tsx`, which now mounts a
 * whole shell of its own, and every query below would be choosing between two
 * grids and two search fields.
 */
function renderAt(component: () => JSX.Element, at: string) {
	const root = createRootRoute({ component });
	const index = createRoute({
		getParentRoute: () => root,
		path: '/',
		component: () => <div data-testid="home-page" />,
	});
	const settings = createRoute({
		getParentRoute: () => root,
		path: '/settings',
		component: () => <div data-testid="settings-page" />,
	});
	// The rail links to all three destinations, so all three have to exist here
	// or `<Link to="/remote">` resolves against a tree that has no such route.
	const remote = createRoute({
		getParentRoute: () => root,
		path: '/remote',
		component: () => <div data-testid="remote-page" />,
	});

	return render(
		<RouterProvider
			router={createRouter({
				routeTree: root.addChildren([index, settings, remote]),
				history: createMemoryHistory({ initialEntries: [at] }),
			})}
		/>
	);
}

/**
 * The rail inside a rendered shell. Queried by the slot the rail marks itself
 * with rather than by position: which child of the grid it is depends on the
 * side, and that is the thing under test here.
 */
function railOf(shell: HTMLElement): HTMLElement {
	const rail = shell.querySelector<HTMLElement>('[data-slot="sidebar"]');
	if (rail === null) throw new Error('the shell rendered no rail');
	return rail;
}

/** The grid, with the routed page in the content column. Returns the grid element. */
async function renderShell(side: SidebarSide, userAgent?: string): Promise<HTMLElement> {
	// The platform decides whether the content column reserves a row at all, so
	// the shell's own grid depends on it — see `useContentHoldsControls`.
	if (userAgent !== undefined) runningOn(userAgent);
	useShellStore.setState({ sidebarSide: side });
	const { container } = renderAt(
		() => (
			<AppShell>
				<Outlet />
			</AppShell>
		),
		'/'
	);
	await screen.findByTestId('home-page');
	return container.querySelector<HTMLElement>('[data-shell]')!;
}

/**
 * The REAL `__root.tsx`, with two ad-hoc children standing in for the app's
 * routes. This is the only way to assert where `MockIndicator` ends up: the
 * question is where `__root` puts it relative to the shell, and a test that
 * re-composed the tree itself would be asserting its own arrangement.
 */
function renderRoot(at: string) {
	const router = createRouter({
		routeTree: appRoot.addChildren([
			createRoute({ getParentRoute: () => appRoot, path: '/', component: () => <div data-testid="home-page" /> }),
			createRoute({
				getParentRoute: () => appRoot,
				path: '/settings',
				component: () => <div data-testid="settings-page" />,
			}),
			createRoute({
				getParentRoute: () => appRoot,
				path: '/remote',
				component: () => <div data-testid="remote-page" />,
			}),
		]),
		history: createMemoryHistory({ initialEntries: [at] }),
	});
	return render(<RouterProvider router={router} />);
}

beforeEach(() => {
	useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
});

afterEach(() => {
	restoreUserAgent();
	// The installed mock is module state, so a case that installs one would
	// otherwise answer for every case after it.
	disposeMock();
	resetBackend();
});

describe('WindowControls on macOS', () => {
	it('marks itself as a drag region', () => {
		// Without this attribute the window has no draggable surface at all —
		// "Overlay" title bar style means macOS no longer provides one.
		const spacer = renderControls(USER_AGENTS.macos);
		expect(spacer?.hasAttribute('data-tauri-drag-region')).toBe(true);
	});

	it('reserves room for the traffic lights, and puts nothing in it', () => {
		// The reserve is a spacer the OS paints over. A glyph, a button or a
		// label here renders underneath three system buttons and is unreachable
		// for the rest of the window's life.
		const spacer = renderControls(USER_AGENTS.macos);
		expect(spacer?.style.width).toBe(`${windowControls()?.width}px`);
		expect(spacer?.childElementCount).toBe(0);
		expect(spacer?.textContent).toBe('');
	});
});

// Finding B: those settings are macOS-only, so Linux and Windows keep the title
// bar the OS draws — with its own controls and its own drag surface. A reserve
// rendered there is 64px of dead width in a cell with nothing to put in it,
// held open for buttons that live somewhere else. The user ruled out replacing
// it with custom window controls: native decorations are the answer on both.
describe('WindowControls off macOS', () => {
	it('renders nothing on Linux, where the OS draws the title bar', () => {
		expect(renderControls(USER_AGENTS.linux)).toBeNull();
	});

	it('renders nothing on Windows, where the OS draws the title bar', () => {
		expect(renderControls(USER_AGENTS.windows)).toBeNull();
	});
});

// The other half of finding B. Nothing in a Tauri build fails when a macOS-only
// key is set on Linux or Windows — tao simply ignores it — so a shared config
// that carries all three reads as though every platform got an overlay title
// bar, which is the assumption that put an unconditional strip in the frontend
// in the first place. Splitting them into tauri.macos.conf.json is what makes
// the platform difference something a reader (and this suite) can see. JSON
// takes no comments, so these assertions are the documentation.
describe('window chrome configuration', () => {
	it('keeps the macOS-only settings out of the shared config', () => {
		for (const key of MACOS_ONLY_WINDOW_KEYS) {
			expect(baseWindow).not.toHaveProperty(key);
		}
	});

	it('applies them in the macOS overlay', () => {
		expect(macosWindow.titleBarStyle).toBe('Overlay');
		expect(macosWindow.hiddenTitle).toBe(true);
		expect(macosWindow.trafficLightPosition).toEqual({ x: 12, y: 20 });
	});

	it('leaves every shared window setting exactly as the shared config has it', () => {
		// Tauri merges a platform config with JSON Merge Patch (RFC 7396), which
		// REPLACES arrays rather than merging them element-wise. So this overlay's
		// `app.windows` stands in for the shared one wholesale, and any key it
		// omits or contradicts silently takes a Tauri default — or a different
		// value — on macOS alone. Duplication is forced; divergence is not, and
		// this is what catches it.
		const shared: Record<string, unknown> = { ...macosWindow };
		for (const key of MACOS_ONLY_WINDOW_KEYS) {
			delete shared[key];
		}
		expect(shared).toEqual(baseWindow);
	});

	it('centres the traffic lights in the row height the shell is built on', () => {
		// `y` is derived, not chosen — but NOT as (ROW_H - 12) / 2, which is what
		// this asserted first and it put the lights visibly high.
		//
		// `y` is not the buttons' top edge. tao's `inset_traffic_lights` resizes
		// the NSTitlebarContainerView to `closeButtonFrameHeight + y` and pins it
		// to the top of the window; AppKit then centres the buttons inside that
		// container. So the visible centre lands at (frame + y) / 2, and centring
		// it on the row means y = ROW_H - frame.
		//
		// The row lives in `--row` and in `ROW_H`, and a JSON file can read
		// neither — retune the scale without this assertion and the lights sit
		// off-centre on macOS alone, with nothing else failing.
		expect(macosWindow.trafficLightPosition.y).toBe(ROW_H - TRAFFIC_LIGHT_FRAME_PX);

		// ...and whatever the frontend puts on that edge still has to clear all
		// three of them, or app chrome renders underneath the buttons. The width
		// comes from `windowControls()` rather than being restated here: a second
		// copy of 64 in this file is one that can be edited on its own, and this
		// assertion would then be checking the copy against itself.
		const spacer = renderControls(USER_AGENTS.macos);
		const reserved = windowControls()?.width ?? 0;
		expect(spacer?.style.width).toBe(`${reserved}px`);
		expect(reserved).toBeGreaterThanOrEqual(macosWindow.trafficLightPosition.x + 3 * TRAFFIC_LIGHT_DIAMETER_PX);
	});
});

// Every window in this app gets its controls from exactly one of two places: the
// OS, or the reserve `WindowControls` opens for them. macOS is the only platform
// where the second is true, and `WindowControls` returning `null` everywhere else
// is only correct while the first holds — so `decorations: false` off macOS
// produces a window with no title bar, no close button, no minimise, and no drag
// surface. Alt+F4 and nothing else. That is precisely what
// `tauri.windows.conf.json` used to do, and the reason these assertions exist:
// custom in-webview window controls were ruled out, so native decorations are the
// only chrome Windows and Linux can have.
describe('native window decorations', () => {
	it.each([
		['linux', USER_AGENTS.linux],
		['windows', USER_AGENTS.windows],
	])('leaves %s the title bar the OS draws, because the frontend draws none', (platform, userAgent) => {
		expect(renderControls(userAgent)).toBeNull();
		// `decorations` defaults to true, so unset is the right way to keep it.
		expect(effectiveWindow(platform).decorations ?? true).toBe(true);
	});

	it('keeps them on macOS too, where the traffic lights are the whole of the chrome', () => {
		// `titleBarStyle: "Overlay"` hides the bar and keeps the buttons —
		// `decorations: false` would take the buttons with it, and then the
		// reserve above would be holding space for nothing.
		expect(effectiveWindow('macos').decorations ?? true).toBe(true);
	});

	it('gives only macOS a platform config at all', () => {
		// An overlay replaces `app.windows` wholesale (see `effectiveWindow`), so
		// one that mentions a single key silently drops title, size, minimums and
		// `backgroundThrottling` on that platform alone — which is what the old
		// `tauri.windows.conf.json` did on top of removing the decorations.
		// macOS is the only platform whose window chrome genuinely differs, so it
		// is the only one that should be paying that price. Anything else needs
		// to restate every shared key deliberately, and this is what makes that
		// decision impossible to make by accident.
		expect(Object.keys(PLATFORM_OVERLAYS)).toEqual(['../../../src-tauri/tauri.macos.conf.json']);
	});
});

describe('the content column’s reserve row', () => {
	/**
	 * The chrome row used to BE the search field. The field is in the rail now,
	 * which leaves this row one job — reserving space for the macOS traffic
	 * lights — and only in the single combination that needs it.
	 */
	it('reserves a row on macOS with the rail on the right', async () => {
		// The one case of the three (spec §4.5): the lights are still on the left
		// edge and the rail has moved off it.
		const shell = await renderShell('right', USER_AGENTS.macos);

		expect(shell.querySelector('[data-slot="window-controls"]')).not.toBeNull();
		expect(shell.className).toContain('grid-rows-[var(--row)_minmax(0,1fr)]');
	});

	it('collapses the row to nothing on macOS with the rail on the left', async () => {
		// The rail holds the reserve there, and holding it here as well opens
		// 64px twice in one window for one set of buttons. With the field gone
		// there is nothing else up here, so the track goes to zero rather than
		// banding the top of the content with 34 empty pixels.
		const shell = await renderShell('left', USER_AGENTS.macos);

		// Scoped to the RAIL: on this platform the reserve does exist, it is just
		// the rail holding it. Querying the whole shell would find that one and
		// pass whatever the content column did.
		expect(railOf(shell).querySelector('[data-slot="window-controls"]')).not.toBeNull();
		expect(shell.querySelector('main [data-slot="window-controls"]')).toBeNull();
		expect(shell.className).toContain('grid-rows-[0_minmax(0,1fr)]');
	});

	it.each([
		['Linux', 'left', USER_AGENTS.linux],
		['Linux', 'right', USER_AGENTS.linux],
		['Windows', 'left', USER_AGENTS.windows],
		['Windows', 'right', USER_AGENTS.windows],
	] as const)('reserves nothing on %s with the rail on the %s', async (_platform, side, userAgent) => {
		// These platforms draw their own title bar, so there is no reserve to
		// hold on either side.
		const shell = await renderShell(side, userAgent);

		expect(shell.querySelector('[data-slot="window-controls"]')).toBeNull();
		expect(shell.className).toContain('grid-rows-[0_minmax(0,1fr)]');
	});
});

describe('AppShell', () => {
	it.each([['left'], ['right']] as const)(
		'runs the rail down the full webview with the rail on the %s',
		async (side) => {
			// `grid-row: 1 / 3` in every combination (spec §1.1). Without the
			// span the rail starts below the chrome row, the divider stops short
			// of the top, and a blank band sits above a rail whose own first row
			// is what the reserve and the history buttons live in.
			const shell = await renderShell(side);
			const rail = railOf(shell);
			expect(shell).toContainElement(rail);
			expect(rail.className).toContain('row-span-2');
		}
	);

	it('puts the rail in column 1 and the content in column 2 with the rail on the left', async () => {
		const shell = await renderShell('left');
		expect(railOf(shell).className).toContain('col-start-1');
		expect(shell.className).toContain('grid-cols-[var(--rail)_minmax(0,1fr)]');
		expect(shell.querySelector('main')?.className).toContain('col-start-2');
	});

	it('reverses the template AND the placement with the rail on the right', async () => {
		// Both halves, because reversing the template alone changes nothing:
		// auto-placement fills row by row, so the rail would stay in track 1 and
		// render as a 246px content column with the real content beside it.
		const shell = await renderShell('right');
		expect(shell.className).toContain('grid-cols-[minmax(0,1fr)_var(--rail)]');
		expect(railOf(shell).className).toContain('col-start-2');
		expect(shell.querySelector('main')?.className).toContain('col-start-1');
	});

	it('leaves the column to the shell rather than letting the rail derive it', async () => {
		// The template and the placements have to reverse TOGETHER. Two copies
		// of the side-to-column mapping drift into the rail auto-placing itself
		// in the `1fr` track: a full-width rail beside a 246px content column,
		// with nothing on screen that looks like a setting.
		const rail = railOf(await renderShell('left'));
		expect(rail.className).toContain('col-start-1');
		expect(rail.className).not.toContain('col-start-2');
	});

	it('puts the reserve row in row 1 of the content column and the outlet in row 2', async () => {
		const shell = await renderShell('right', USER_AGENTS.macos);
		const reserve = shell.querySelector('[data-slot="window-controls"]');

		expect(reserve?.closest('[data-shell] > *')?.className).toContain('row-start-1');
		expect(shell.querySelector('main')?.className).toContain('row-start-2');
	});

	it('paints the content column, and the reserve row with it', async () => {
		// The rail is a different surface from the content column, and the
		// reserve row belongs to the content — unpainted, the 34px above the
		// outlet shows the window's own ground and reads as a seam.
		const shell = await renderShell('right', USER_AGENTS.macos);
		const reserveCell = shell.querySelector('[data-slot="window-controls"]')?.closest('[data-shell] > *');

		expect(reserveCell?.className).toContain('bg-background');
		expect(shell.querySelector('main')?.className).toContain('bg-background');
	});

	it('answers the selector ResizeHandle resolves its write target with', async () => {
		// Verbatim from `shellOf` in `sidebar/components/resize-handle.tsx`. Drop
		// `data-shell` and a drag silently retargets `document.documentElement`,
		// where the inline `--rail` below outranks whatever it writes.
		const shell = await renderShell('left');
		expect(shell.matches('[data-shell], [style*="--rail"]')).toBe(true);
	});

	it('declares --rail inline from the store, not only on :root', async () => {
		// An inline declaration on this element outranks the `:root` rule in
		// index.css. That is the whole reason the resize handle writes here: with
		// the property left on the root alone, this style would overrule sixty
		// writes a second and the rail would sit still through the drag, then
		// snap to its final width on pointer-up.
		useShellStore.setState({ sidebarWidth: 200 });
		const shell = await renderShell('left');
		expect(shell.style.getPropertyValue('--rail')).toBe('200px');
	});
});

describe('the root layout', () => {
	it('puts the mock banner at the bottom of the content column, clear of the rail', async () => {
		// Two placements were tried and both were wrong on screen. Spanning the
		// window ran it across the rail and up against the traffic lights; between
		// the chrome row and the page it cut the shell in half. It belongs to the
		// page, so it sits under the page — in the content column's own track,
		// where it stops at the divider.
		installMock('normal');
		const { container } = renderRoot('/');

		const badge = await screen.findByText('Mock');
		const main = container.querySelector<HTMLElement>('main');

		expect(main).toContainElement(badge);
		expect(container.querySelector('[data-slot="sidebar"]')).not.toContainElement(badge);
		// Last child of the column, so the scroll region above it can grow and
		// the band stays put.
		expect(main?.lastElementChild).toContainElement(badge);
	});

	it('scrolls the page above the banner rather than scrolling the banner away', async () => {
		// The overflow belongs to the wrapper, not to the cell. On the cell, the
		// band is inside the scrolling box and leaves the bottom of the window the
		// moment the page is taller than the column.
		installMock('normal');
		const { container } = renderRoot('/');

		await screen.findByText('Mock');
		const main = container.querySelector<HTMLElement>('main');

		expect(main?.className).not.toContain('overflow-auto');
		expect(main?.firstElementChild?.className).toContain('overflow-auto');
		expect(main?.firstElementChild?.className).toContain('flex-1');
	});

	it('gives the grid the whole window again, now that nothing stacks above it', async () => {
		// The band used to be a row of the window, which forced the grid to take
		// `flex-1` instead. It is a cell's child now, so the shell is the window.
		installMock('normal');
		const { container } = renderRoot('/');

		await screen.findByText('Mock');
		const shell = container.querySelector<HTMLElement>('[data-shell]');

		expect(shell?.className).toContain('h-screen');
	});

	it('keeps the mock banner out of the chrome row', async () => {
		// A dev-only band has no business in the window chrome, and in row 1 it
		// would be sharing a `--row`-tall track with the search field.
		installMock('normal');
		renderRoot('/');

		await screen.findByText('Mock');
		const chromeCell = screen.getByRole('textbox', { name: 'Search' }).closest('[data-shell] > *');
		expect(chromeCell).not.toContainElement(screen.getByText('Mock'));
	});
});
