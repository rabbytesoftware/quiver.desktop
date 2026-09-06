import type { JSX } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
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

vi.mock('@tanstack/react-router-devtools', () => ({ TanStackRouterDevtools: () => null }));

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import { disposeMock, installMock } from '@/lib/mock';
import { resetBackend } from '@/lib/transport/backend';
import { Route as appRoot } from '@/routes/__root';

import { AppShell } from './components/app-shell';
import { WindowControls } from './components/window-controls';
import { ROW_H, windowControls, type SidebarSide } from './lib/geometry';
import { SIDEBAR_DEFAULT, useShellStore } from './stores/shell-store';
import baseConfig from '../../../src-tauri/tauri.conf.json';
import macosConfig from '../../../src-tauri/tauri.macos.conf.json';

const MACOS_ONLY_WINDOW_KEYS = ['titleBarStyle', 'hiddenTitle', 'trafficLightPosition'];

const baseWindow = baseConfig.app.windows[0] as Record<string, unknown>;
const macosWindow = macosConfig.app.windows[0];

const PLATFORM_OVERLAYS = import.meta.glob<{ app: { windows: Record<string, unknown>[] } }>(
	'../../../src-tauri/tauri.*.conf.json',
	{ eager: true, import: 'default' }
);

function effectiveWindow(platform: string): Record<string, unknown> {
	return PLATFORM_OVERLAYS[`../../../src-tauri/tauri.${platform}.conf.json`]?.app.windows[0] ?? baseWindow;
}

const TRAFFIC_LIGHT_DIAMETER_PX = 12;

const TRAFFIC_LIGHT_FRAME_PX = 14;

function renderControls(userAgent: string): HTMLElement | null {
	runningOn(userAgent);
	return render(<WindowControls />).container.querySelector('[data-slot="window-controls"]');
}

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

function railOf(shell: HTMLElement): HTMLElement {
	const rail = shell.querySelector<HTMLElement>('[data-slot="sidebar"]');
	if (rail === null) throw new Error('the shell rendered no rail');
	return rail;
}

async function renderShell(side: SidebarSide, userAgent?: string): Promise<HTMLElement> {
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
	// The root layout now mounts `CommandPalette`, which -- like the rest of
	// the app -- needs a QueryClientProvider ancestor for its mutations.
	const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	return render(
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	);
}

beforeEach(() => {
	useShellStore.setState({ sidebarSide: 'left', sidebarWidth: SIDEBAR_DEFAULT });
});

afterEach(() => {
	restoreUserAgent();
	disposeMock();
	resetBackend();
});

describe('WindowControls on macOS', () => {
	it('marks itself as a drag region', () => {
		const spacer = renderControls(USER_AGENTS.macos);
		expect(spacer?.hasAttribute('data-tauri-drag-region')).toBe(true);
	});

	it('reserves room for the traffic lights, and puts nothing in it', () => {
		const spacer = renderControls(USER_AGENTS.macos);
		expect(spacer?.style.width).toBe(`${windowControls()?.width}px`);
		expect(spacer?.childElementCount).toBe(0);
		expect(spacer?.textContent).toBe('');
	});
});

describe('WindowControls off macOS', () => {
	it('renders nothing on Linux, where the OS draws the title bar', () => {
		expect(renderControls(USER_AGENTS.linux)).toBeNull();
	});

	it('renders nothing on Windows, where the OS draws the title bar', () => {
		expect(renderControls(USER_AGENTS.windows)).toBeNull();
	});
});

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
		const shared: Record<string, unknown> = { ...macosWindow };
		for (const key of MACOS_ONLY_WINDOW_KEYS) {
			delete shared[key];
		}
		expect(shared).toEqual(baseWindow);
	});

	it('centres the traffic lights in the row height the shell is built on', () => {
		expect(macosWindow.trafficLightPosition.y).toBe(ROW_H - TRAFFIC_LIGHT_FRAME_PX);

		const spacer = renderControls(USER_AGENTS.macos);
		const reserved = windowControls()?.width ?? 0;
		expect(spacer?.style.width).toBe(`${reserved}px`);
		expect(reserved).toBeGreaterThanOrEqual(macosWindow.trafficLightPosition.x + 3 * TRAFFIC_LIGHT_DIAMETER_PX);
	});
});

describe('native window decorations', () => {
	it.each([
		['linux', USER_AGENTS.linux],
		['windows', USER_AGENTS.windows],
	])('leaves %s the title bar the OS draws, because the frontend draws none', (platform, userAgent) => {
		expect(renderControls(userAgent)).toBeNull();
		expect(effectiveWindow(platform).decorations ?? true).toBe(true);
	});

	it('keeps them on macOS too, where the traffic lights are the whole of the chrome', () => {
		expect(effectiveWindow('macos').decorations ?? true).toBe(true);
	});

	it('gives only macOS a platform config at all', () => {
		expect(Object.keys(PLATFORM_OVERLAYS)).toEqual(['../../../src-tauri/tauri.macos.conf.json']);
	});
});

describe('the content column’s reserve row', () => {
	it('reserves a row on macOS with the rail on the right', async () => {
		const shell = await renderShell('right', USER_AGENTS.macos);

		expect(shell.querySelector('[data-slot="window-controls"]')).not.toBeNull();
		expect(shell.className).toContain('grid-rows-[var(--row)_minmax(0,1fr)]');
	});

	it('collapses the row to nothing on macOS with the rail on the left', async () => {
		const shell = await renderShell('left', USER_AGENTS.macos);

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
		const shell = await renderShell(side, userAgent);

		expect(shell.querySelector('[data-slot="window-controls"]')).toBeNull();
		expect(shell.className).toContain('grid-rows-[0_minmax(0,1fr)]');
	});
});

describe('AppShell', () => {
	it.each([['left'], ['right']] as const)(
		'runs the rail down the full webview with the rail on the %s',
		async (side) => {
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
		const shell = await renderShell('right');
		expect(shell.className).toContain('grid-cols-[minmax(0,1fr)_var(--rail)]');
		expect(railOf(shell).className).toContain('col-start-2');
		expect(shell.querySelector('main')?.className).toContain('col-start-1');
	});

	it('leaves the column to the shell rather than letting the rail derive it', async () => {
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
		const shell = await renderShell('right', USER_AGENTS.macos);
		const reserveCell = shell.querySelector('[data-slot="window-controls"]')?.closest('[data-shell] > *');

		expect(reserveCell?.className).toContain('bg-background');
		expect(shell.querySelector('main')?.className).toContain('bg-background');
	});

	it('answers the selector ResizeHandle resolves its write target with', async () => {
		const shell = await renderShell('left');
		expect(shell.matches('[data-shell], [style*="--rail"]')).toBe(true);
	});

	it('declares --rail inline from the store, not only on :root', async () => {
		useShellStore.setState({ sidebarWidth: 200 });
		const shell = await renderShell('left');
		expect(shell.style.getPropertyValue('--rail')).toBe('200px');
	});
});

describe('the root layout', () => {
	it('puts the mock banner at the bottom of the content column, clear of the rail', async () => {
		installMock('normal');
		const { container } = renderRoot('/');

		const badge = await screen.findByText('Mock');
		const main = container.querySelector<HTMLElement>('main');

		expect(main).toContainElement(badge);
		expect(container.querySelector('[data-slot="sidebar"]')).not.toContainElement(badge);
		expect(main?.lastElementChild).toContainElement(badge);
	});

	it('scrolls the page above the banner rather than scrolling the banner away', async () => {
		installMock('normal');
		const { container } = renderRoot('/');

		await screen.findByText('Mock');
		const main = container.querySelector<HTMLElement>('main');

		expect(main?.className).not.toContain('overflow-auto');
		expect(main?.firstElementChild?.className).toContain('overflow-auto');
		expect(main?.firstElementChild?.className).toContain('flex-1');
	});

	it('gives the grid the whole window again, now that nothing stacks above it', async () => {
		installMock('normal');
		const { container } = renderRoot('/');

		await screen.findByText('Mock');
		const shell = container.querySelector<HTMLElement>('[data-shell]');

		expect(shell?.className).toContain('h-screen');
	});

	it('keeps the mock banner out of the chrome row', async () => {
		installMock('normal');
		renderRoot('/');

		await screen.findByText('Mock');
		const chromeCell = screen.getByRole('textbox', { name: 'Search' }).closest('[data-shell] > *');
		expect(chromeCell).not.toContainElement(screen.getByText('Mock'));
	});
});
