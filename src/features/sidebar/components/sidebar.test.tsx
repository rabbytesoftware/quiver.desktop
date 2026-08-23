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

import { Sidebar } from './sidebar';

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
	await screen.findAllByRole('tab');

	const rail = container.querySelector<HTMLElement>('[data-slot="sidebar"]');
	if (rail === null) throw new Error('the rail did not render');
	return { rail, router };
}

async function openArrow(router: Awaited<ReturnType<typeof renderRail>>['router'], namespace: string) {
	await act(async () => {
		await router.navigate({ to: '/arrow/$', params: { _splat: namespace } });
	});
	await screen.findByTestId('page-arrow');
}

function activeNames(rail: HTMLElement): string[] {
	return Array.from(rail.querySelectorAll('[data-status="active"]')).map(
		(row) => row.getAttribute('aria-label') ?? row.querySelector('[data-slot="arrow-name"]')?.textContent ?? ''
	);
}

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
	restoreUserAgent();
});

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

	it('lights nothing anywhere at /search', async () => {
		const { rail } = await renderRail('/search');
		expect(activeNames(rail)).toEqual([]);
	});
});

describe('Sidebar', () => {
	it('spans both grid rows so no blank band sits above the rail', async () => {
		const { rail } = await renderRail('/');
		expect(rail.className).toContain('row-span-2');
	});

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
		const { rail } = await renderRail('/', side);
		expect(rail.className).toContain(faces);
		expect(rail.className).not.toContain(away);
	});

	it('scrolls the arrow list and nothing above it', async () => {
		useArrowStore.setState({ arrows: new Map([[MINECRAFT, entry(MINECRAFT, 'Minecraft')]]) });
		const { rail } = await renderRail('/');

		const scroller = rail.querySelector('[data-slot="scroll-area"]');
		expect(scroller).toContainElement(screen.getByRole('navigation', { name: 'Arrows' }));
		expect(scroller).not.toContainElement(topBar());
		expect(scroller).not.toContainElement(screen.getByRole('tab', { name: 'Home' }));
	});
});

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

		expect(topBar().firstElementChild).toHaveAttribute('data-slot', 'window-controls');
		expect(topBar().lastElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('leaves the reserve to the chrome row on macOS with the rail on the right', async () => {
		runningOn(USER_AGENTS.macos);
		await renderRail('/', 'right');
		expect(topBar().querySelector('[data-slot="window-controls"]')).toBeNull();
	});

	it.each([
		['Linux', USER_AGENTS.linux],
		['Windows', USER_AGENTS.windows],
	])('reserves nothing on %s, where the OS draws its own title bar', async (_platform, userAgent) => {
		runningOn(userAgent);
		await renderRail('/', 'left');

		expect(topBar().querySelector('[data-slot="window-controls"]')).toBeNull();
		expect(topBar().lastElementChild).toContainElement(screen.getByRole('button', { name: 'Go back' }));
	});

	it('is one row tall, from the token rather than from pixels', async () => {
		await renderRail('/');
		expect(topBar().className).toContain('h-(--row)');
	});

	it('makes the whole row a window handle, leaving the history buttons clickable', async () => {
		await renderRail('/', 'left');
		const bar = topBar();

		expect(bar).toHaveAttribute('data-tauri-drag-region');
		expect(bar.querySelector('.flex-1')).toHaveAttribute('data-tauri-drag-region');
		for (const label of ['Go back', 'Go forward']) {
			expect(screen.getByRole('button', { name: label })).not.toHaveAttribute('data-tauri-drag-region');
		}
	});
});

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

		await user.click(screen.getByRole('tab', { name: 'Settings' }));

		expect(router.history.length).toBe(before);
		expect(router.state.location.searchStr).toBe('?tab=developer');
	});

	it('still pushes when a different destination is clicked', async () => {
		const user = userEvent.setup();
		const { router } = await renderRail('/settings?tab=developer');

		await user.click(screen.getByRole('tab', { name: 'Home' }));

		expect(await screen.findByTestId('page-home')).toBeInTheDocument();
		expect(router.history.canGoBack()).toBe(true);
	});

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
