import type { ReactNode } from 'react';

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

import { SearchBar } from './index';

interface Slots {
	leading?: ReactNode;
	trailing?: ReactNode;
}

/**
 * The field goes in the ROOT component, which is where the chrome row puts it —
 * above the `<Outlet/>`, so nothing re-renders it when the route changes. Under
 * a leaf route it would remount on every navigation and a field that reads its
 * value from a stale closure would look reactive.
 *
 * `/search` restates `src/routes/search.tsx`'s `validateSearch` rather than
 * importing the real route tree: `__root.tsx` mounts the whole shell, whose
 * chrome row is this very component — every query below would then be choosing
 * between two search fields, and the slot assertions would be reading the one
 * that was passed nothing.
 */
async function renderField(slots: Slots = {}, initialEntries = ['/']) {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				<SearchBar {...slots} />
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
	it('placeholds with exactly "Search"', async () => {
		const { input } = await renderField();
		expect(input).toHaveAttribute('placeholder', 'Search');
	});

	it('names the field for screen readers, which have no visible label to read', async () => {
		const { input } = await renderField();
		expect(input).toHaveAttribute('aria-label', 'Search');
	});

	it('shows the keyboard hint', async () => {
		await renderField();
		expect(screen.getByText('⌘K')).toBeInTheDocument();
	});

	it('puts what is typed into ?q= on /search', async () => {
		const { input, router, user } = await renderField();

		await user.type(input, 'minecraft');

		expect(await screen.findByTestId('search-page')).toBeInTheDocument();
		expect(router.state.location.pathname).toBe('/search');
		expect(router.state.location.search).toEqual({ q: 'minecraft' });
	});

	it('reflects the URL back into the field, so the back button cannot desync it', async () => {
		const { input } = await renderField({}, ['/search?q=minecraft']);
		expect(input).toHaveValue('minecraft');
	});

	/**
	 * `/search` with no `?q=` at all is reachable from the address bar and from
	 * any link that forgot the param. The field has to read that as an empty
	 * query; `null` reaches `value` as an uncontrolled input and React warns
	 * once, then stops, and the field silently stops tracking the URL.
	 */
	it('reads a missing ?q= as an empty query', async () => {
		const { input } = await renderField({}, ['/search']);
		expect(input).toHaveValue('');
	});

	/**
	 * The count is the assertion, not the spy. `replace: true` still calls
	 * `navigate`, so a spy sees nine calls whether every keystroke pushes or
	 * only the first does — and nine entries is exactly the bug: back walks the
	 * query backwards one character at a time instead of leaving /search.
	 */
	it('pushes the first navigation and replaces every keystroke after it', async () => {
		const { input, router, user } = await renderField();
		expect(router.history.length).toBe(1);

		await user.type(input, 'm');
		expect(router.state.location.pathname).toBe('/search');
		expect(router.history.length).toBe(2);

		await user.type(input, 'inecraft');
		expect(router.state.location.search).toEqual({ q: 'minecraft' });
		expect(router.history.length).toBe(2);
	});

	/**
	 * The unchanged length is the half that matters. Landing back on `/` proves
	 * nothing on its own — `navigate({ to: '/' })` gets there too, and pushes a
	 * third entry doing it, so back would then return to the emptied `/search`
	 * the user just left. Popping the entry the first keystroke pushed is what
	 * leaves history the length it was before the field was ever touched.
	 */
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

	it('renders both slots', async () => {
		await renderField({
			leading: <div data-testid="leading" />,
			trailing: <div data-testid="trailing" />,
		});

		expect(screen.getByTestId('leading')).toBeInTheDocument();
		expect(screen.getByTestId('trailing')).toBeInTheDocument();
	});

	/**
	 * The doubled gap is invisible to jsdom, which has no layout: the caller's
	 * own inset and the field's stack, and the only place that can be caught is
	 * the class list.
	 */
	it('keeps its own inset on both sides when no slot is given', async () => {
		const { input } = await renderField();
		expect(input.className).toContain('pl-(--inset)');
		expect(input.className).toContain('pr-(--inset)');
	});

	it('drops the padding on whichever side a slot supplies its own inset', async () => {
		const { input } = await renderField({
			leading: <div data-testid="leading" />,
			trailing: <div data-testid="trailing" />,
		});

		expect(input.className).toContain('pl-0');
		expect(input.className).not.toContain('pl-(--inset)');
		expect(input.className).toContain('pr-0');
		expect(input.className).not.toContain('pr-(--inset)');
	});

	/**
	 * `h-[34px]` and `h-(--row)` look identical today and stop tracking each
	 * other the moment the row height moves, with no layout assertion possible
	 * in jsdom to notice. The inversion is checked the same way and for the same
	 * reason — and `--primary` must not appear, or the field settles the accent
	 * question the palette has not.
	 */
	it('takes its height and its focused inversion from the tokens', async () => {
		const { input } = await renderField();
		const plate = input.parentElement;

		expect(plate?.className).toContain('h-(--row)');
		expect(plate?.className).toContain('focus-within:bg-foreground');
		expect(plate?.className).toContain('focus-within:text-background');
		expect(plate?.className).not.toContain('primary');
	});
});
