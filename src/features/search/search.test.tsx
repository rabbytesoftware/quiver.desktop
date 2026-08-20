import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { LOCAL_DEBOUNCE_MS, SearchBar } from './index';

async function renderField(initialEntries = ['/']) {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				<SearchBar />
				<Outlet />
			</>
		),
	});
	const indexRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home" />,
	});
	const searchRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/search',
		validateSearch: (search: Record<string, unknown>) => ({ q: typeof search.q === 'string' ? search.q : '' }),
		component: () => <div data-testid="search-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, searchRoute]),
		history: createMemoryHistory({ initialEntries }),
	});

	render(<RouterProvider router={router} />);

	const input = await screen.findByRole('textbox', { name: 'Search' });
	const user = userEvent.setup();
	return { input, router, user };
}

describe('SearchBar', () => {
	it('names what is searched, not the act of searching', async () => {
		const { input } = await renderField();
		expect(input).toHaveAttribute('placeholder', 'Search Arrows');
	});

	it('names the field for screen readers, which have no visible label to read', async () => {
		const { input } = await renderField();
		expect(input).toHaveAttribute('aria-label', 'Search');
	});

	it('shows no keyboard hint', async () => {
		await renderField();
		expect(screen.queryByText('⌘K')).toBeNull();
	});

	it('puts what is typed into ?q= on /search', async () => {
		const { input, router, user } = await renderField();

		await user.type(input, 'minecraft');

		expect(await screen.findByTestId('search-page')).toBeInTheDocument();
		expect(router.state.location.pathname).toBe('/search');
		await waitFor(() => expect(router.state.location.search).toEqual({ q: 'minecraft' }));
	});

	it('reflects the URL back into the field, so the back button cannot desync it', async () => {
		const { input } = await renderField(['/search?q=minecraft']);
		expect(input).toHaveValue('minecraft');
	});

	it('reads a missing ?q= as an empty query', async () => {
		const { input } = await renderField(['/search']);
		expect(input).toHaveValue('');
	});

	it('pushes the first navigation and replaces every keystroke after it', async () => {
		const { input, router, user } = await renderField();
		expect(router.history.length).toBe(1);

		await user.type(input, 'm');
		expect(router.state.location.pathname).toBe('/search');
		expect(router.history.length).toBe(2);

		await user.type(input, 'inecraft');
		await waitFor(() => expect(router.state.location.search).toEqual({ q: 'minecraft' }));
		expect(router.history.length).toBe(2);
	});

	it('leaves /search when the query is emptied, by popping the entry it pushed', async () => {
		const { input, router, user } = await renderField();

		await user.type(input, 'minecraft');
		expect(router.state.location.pathname).toBe('/search');
		expect(router.history.length).toBe(2);

		await user.clear(input);

		expect(await screen.findByTestId('home')).toBeInTheDocument();
		expect(router.state.location.pathname).toBe('/');
		expect(router.history.length).toBe(2);
	});

	it('opens with the lens, ahead of the input', async () => {
		const { input } = await renderField();
		const lens = input.parentElement?.firstElementChild;

		expect(lens?.querySelector('svg')).not.toBeNull();
		expect(lens?.nextElementSibling).toBe(input);
	});

	it('strokes the lens in currentColor, so it inverts with the plate', async () => {
		const lens = (await renderField()).input.parentElement?.querySelector('svg');

		expect(lens?.querySelector('circle')?.getAttribute('stroke')).toBe('currentColor');
		expect(lens?.querySelector('path')?.getAttribute('stroke')).toBe('currentColor');
	});

	it('navigates to /search when the field is focused', async () => {
		const { input, router } = await renderField();
		expect(router.state.location.pathname).toBe('/');

		input.focus();
		await screen.findByTestId('search-page');

		expect(router.state.location.pathname).toBe('/search');
	});

	it('does not push another entry when focused again on /search', async () => {
		const { input } = await renderField(['/search?q=redis']);
		const before = window.history.length;

		input.focus();
		input.blur();
		input.focus();

		expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('redis');
		expect(window.history.length).toBe(before);
	});

	it('marks itself active on the results route', async () => {
		const field = (await renderField(['/search?q=x'])).input.parentElement;
		expect(field).toHaveAttribute('data-active');
	});

	it('is not marked active anywhere else', async () => {
		const field = (await renderField()).input.parentElement;
		expect(field).not.toHaveAttribute('data-active');
	});

	it('is not a window drag handle', async () => {
		const { input } = await renderField();

		expect(input.parentElement).not.toHaveAttribute('data-tauri-drag-region');
		expect(input).not.toHaveAttribute('data-tauri-drag-region');
	});

	it('waits out the debounce before putting a keystroke in the URL', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const { input, router, user } = await renderField(['/search?q=']);

		await user.type(input, 'serv');
		expect(router.state.location.search).toEqual({ q: '' });

		await vi.advanceTimersByTimeAsync(LOCAL_DEBOUNCE_MS + 10);
		expect(router.state.location.search).toEqual({ q: 'serv' });

		vi.useRealTimers();
	});

	it('shows what was typed immediately, even before it reaches the URL', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const { input, user } = await renderField(['/search?q=']);

		await user.type(input, 'serv');
		expect((input as HTMLInputElement).value).toBe('serv');

		vi.useRealTimers();
	});

	it('commits immediately on Enter rather than waiting out the debounce', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const { input, router, user } = await renderField(['/search?q=']);

		await user.type(input, 'serv{Enter}');
		expect(router.state.location.search).toEqual({ q: 'serv' });

		vi.useRealTimers();
	});

	it('does not let a stale debounce undo a back-navigation that lands before it fires', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const { input, router, user } = await renderField(['/', '/search?q=x']);
		expect(router.history.length).toBe(2);

		await user.type(input, 'y');
		expect((input as HTMLInputElement).value).toBe('xy');

		await act(async () => {
			router.history.back();
		});
		expect(router.state.location.pathname).toBe('/');

		await vi.advanceTimersByTimeAsync(LOCAL_DEBOUNCE_MS + 10);
		expect(router.state.location.pathname).toBe('/');
		expect(router.history.length).toBe(2);

		vi.useRealTimers();
	});
});
