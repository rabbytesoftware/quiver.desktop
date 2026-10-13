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

import { HistoryNav } from './history-nav';

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

	it('takes its hover from the rail’s element token, not the button default', async () => {
		const { back, forward } = await renderNav();
		for (const button of [back, forward]) {
			expect(button.className).toContain('hover:bg-sidebar-element-hover');
			expect(button.className).toContain('rounded-sm');
			expect(button.querySelector('svg')).toHaveAttribute('width', '16');
		}
	});
});
