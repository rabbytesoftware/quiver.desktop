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

/**
 * The field goes in the ROOT component, which is where the rail puts it —
 * above the `<Outlet/>`, so nothing re-renders it when the route changes. Under
 * a leaf route it would remount on every navigation and a field that reads its
 * value from a stale closure would look reactive.
 *
 * `/search` restates `src/routes/search.tsx`'s `validateSearch` rather than
 * importing the real route tree: `__root.tsx` mounts the whole shell, whose
 * rail is where this component now lives — every query below would then be
 * choosing between two search fields.
 */
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
		const { input } = await renderField(['/search?q=minecraft']);
		expect(input).toHaveValue('minecraft');
	});

	/**
	 * `/search` with no `?q=` at all is reachable from the address bar and from
	 * any link that forgot the param. The field has to read that as an empty
	 * query; `null` reaches `value` as an uncontrolled input and React warns
	 * once, then stops, and the field silently stops tracking the URL.
	 */
	it('reads a missing ?q= as an empty query', async () => {
		const { input } = await renderField(['/search']);
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

	/**
	 * The lens went missing once already, in a shell that still looked plausible
	 * because the placeholder rendered — so it is asserted by position rather
	 * than by presence: leading edge first, input after it.
	 */
	it('opens with the lens, ahead of the input', async () => {
		const { input } = await renderField();
		const lens = input.parentElement?.firstElementChild;

		expect(lens?.querySelector('svg')).not.toBeNull();
		expect(lens?.nextElementSibling).toBe(input);
	});

	it('strokes the lens in currentColor, so it inverts with the plate', async () => {
		// A literal colour survives the inversion and disappears into it: a
		// white magnifier on the white focused plate, with the field otherwise
		// working perfectly.
		const lens = (await renderField()).input.parentElement?.querySelector('svg');

		expect(lens?.querySelector('circle')?.getAttribute('stroke')).toBe('currentColor');
		expect(lens?.querySelector('path')?.getAttribute('stroke')).toBe('currentColor');
	});

	/**
	 * The field is a DESTINATION now, not a control that happens to sit above the
	 * content (spec §1.6). It lives in the rail beside Home, Remote and Settings,
	 * and reaching it is a navigation like reaching any of them.
	 *
	 * On `focus` rather than `click`, so the keyboard arrives at the same place —
	 * tabbing in, and the shortcut the field advertises, both land here without a
	 * second handler that could drift from this one.
	 */
	it('navigates to /search when the field is focused', async () => {
		const { input, router } = await renderField();
		expect(router.state.location.pathname).toBe('/');

		input.focus();
		await screen.findByTestId('search-page');

		expect(router.state.location.pathname).toBe('/search');
	});

	/**
	 * Without the guard every re-focus pushes another entry, and the back button
	 * then walks a stack of identical URLs instead of leaving.
	 */
	it('does not push another entry when focused again on /search', async () => {
		const { input } = await renderField(['/search?q=redis']);
		const before = window.history.length;

		input.focus();
		input.blur();
		input.focus();

		expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('redis');
		expect(window.history.length).toBe(before);
	});

	/**
	 * The same inversion the rail uses everywhere else to say "this is where you
	 * are" — a selected arrow row, and the changer's active segment. Marked with
	 * `data-active` rather than a class so the styling stays in one place.
	 */
	it('marks itself active on the results route', async () => {
		const field = (await renderField(['/search?q=x'])).input.parentElement;
		expect(field).toHaveAttribute('data-active');
	});

	it('is not marked active anywhere else', async () => {
		// Two renders in one test would leave two fields mounted and every
		// `screen` query ambiguous, so the pair is split.
		const field = (await renderField()).input.parentElement;
		expect(field).not.toHaveAttribute('data-active');
	});

	/**
	 * The window-drag gesture went with the move into the rail: a drag region
	 * fires on `mousedown` with no threshold, which would take the press away
	 * from the navigation above. `RailTopBar` is the handle now.
	 */
	it('is not a window drag handle', async () => {
		const { input } = await renderField();

		expect(input.parentElement).not.toHaveAttribute('data-tauri-drag-region');
		expect(input).not.toHaveAttribute('data-tauri-drag-region');
	});
});
