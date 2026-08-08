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
import { describe, expect, it, vi } from 'vitest';

import { HistoryNav } from './components/history-nav';

/**
 * A two-route tree with the nav in the ROOT component, which is where the rail
 * puts it. Rendering it under a leaf route would hide the interesting failure:
 * a leaf re-renders on every navigation, so a stale `canGoBack` read would look
 * reactive and the disabled state would appear to work.
 */
async function renderNav() {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				<HistoryNav />
				<Outlet />
			</>
		),
	});
	const indexRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home" />,
	});
	const remoteRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/remote',
		component: () => <div data-testid="remote" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, remoteRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	render(<RouterProvider router={router} />);

	const back = await screen.findByRole('button', { name: 'Go back' });
	const forward = await screen.findByRole('button', { name: 'Go forward' });
	return { back, forward, router };
}

describe('HistoryNav', () => {
	it('names both buttons for screen readers', async () => {
		const { back, forward } = await renderNav();
		expect(back).toHaveAttribute('aria-label', 'Go back');
		expect(forward).toHaveAttribute('aria-label', 'Go forward');
	});

	it('disables back at the start of history', async () => {
		const { back } = await renderNav();
		expect(back).toBeDisabled();
	});

	it('enables back once a navigation has happened', async () => {
		const { back, router } = await renderNav();
		await act(async () => {
			await router.navigate({ to: '/remote' });
		});
		expect(await screen.findByTestId('remote')).toBeInTheDocument();
		expect(back).toBeEnabled();
	});

	it('drives the router when back is clicked', async () => {
		const user = userEvent.setup();
		const { back, router } = await renderNav();
		await act(async () => {
			await router.navigate({ to: '/remote' });
		});
		const goBack = vi.spyOn(router.history, 'back');

		await user.click(back);
		expect(goBack).toHaveBeenCalled();
	});

	// No `canGoForward()` exists to ask, and shadow-tracking an index to answer
	// it would be a second copy of the router's state, free to drift.
	it('never disables forward', async () => {
		const { forward, router } = await renderNav();
		expect(forward).toBeEnabled();

		await act(async () => {
			await router.navigate({ to: '/remote' });
		});
		expect(forward).toBeEnabled();

		await act(async () => {
			router.history.back();
		});
		expect(forward).toBeEnabled();
	});

	it('drives the router when forward is clicked', async () => {
		const user = userEvent.setup();
		const { forward, router } = await renderNav();
		const goForward = vi.spyOn(router.history, 'forward');

		await user.click(forward);
		expect(goForward).toHaveBeenCalled();
	});

	/**
	 * The failure this catches is silent: `size-[34px]` and `size-8` both LOOK
	 * right today and stop tracking `--row` the moment it moves, with no layout
	 * assertion possible in jsdom to notice.
	 */
	it('sizes itself from the geometry tokens rather than from pixels', async () => {
		const { back, forward } = await renderNav();
		for (const button of [back, forward]) {
			expect(button.className).toContain('size-(--row)');
			expect(button.querySelector('svg')?.getAttribute('class')).toContain('size-(--icon-chrome)');
		}
	});
});
