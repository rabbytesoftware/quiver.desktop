// Hoisted so the spy exists before the module factory runs and the tests can
// assert on the same function the component calls.
const { startDragging } = vi.hoisted(() => ({ startDragging: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ startDragging }) }));
import type { ReactNode } from 'react';

import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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
	 *
	 * On the PLATE and not on the input, which is the half that was wrong: the
	 * lens sits ahead of the input, so an inset worn by the input alone leaves
	 * the magnifier flush against the window's edge.
	 */
	it('keeps its own 12px inset on both sides when no slot is given', async () => {
		const plate = (await renderField()).input.parentElement;
		expect(plate?.className).toContain('pl-[12px]');
		expect(plate?.className).toContain('pr-[12px]');
	});

	it('drops the padding on whichever side a slot supplies its own inset', async () => {
		const { input } = await renderField({
			leading: <div data-testid="leading" />,
			trailing: <div data-testid="trailing" />,
		});
		const plate = input.parentElement;

		expect(plate?.className).toContain('pl-0');
		expect(plate?.className).not.toContain('pl-[12px]');
		expect(plate?.className).toContain('pr-0');
		expect(plate?.className).not.toContain('pr-[12px]');
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
	 * Both focus behaviours are invisible to jsdom — it applies no `:focus-within`
	 * rule and composites nothing — so the class list is the only place they can
	 * be caught, and both were missing from the first build.
	 */
	it('hides the hint and drops the blur on focus', async () => {
		const { input } = await renderField();

		// The hint explains how to reach a field you are already typing in, and
		// it is the one thing left on the plate competing with the query.
		expect(screen.getByText('⌘K').className).toContain('group-focus-within:hidden');
		// The focused plate is opaque, so the blur is a compositor pass over
		// every scrolled frame underneath that paints nothing anyone can see.
		expect(input.parentElement?.className).toContain('focus-within:backdrop-filter-none');
	});

	/**
	 * None of these sizes has a token — they are the design's own, and the only
	 * record of them outside `design.pen` is the class list.
	 */
	it('types the query at 12/480 and the hint at 9.5, with an italic placeholder', async () => {
		const { input } = await renderField();

		expect(input.className).toContain('text-[12px]');
		expect(input.className).toContain('font-[480]');
		expect(input.className).toContain('placeholder:italic');
		expect(screen.getByText('⌘K').className).toContain('text-[9.5px]');
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

	/**
	 * macOS hides its title bar under `titleBarStyle: "Overlay"` and takes every
	 * draggable surface with it, so a chrome row that is not a drag region leaves
	 * the top of the window dead — nothing about the page looks wrong, the window
	 * simply cannot be moved from there.
	 *
	 * The input must NOT carry it. Tauri dispatches on the event target, so the
	 * attribute on the field would take mousedown away from focusing it and from
	 * selecting the text already in it — and the failure reads as "the search box
	 * is broken", not as "the drag region is too greedy".
	 */
	it('makes the plate a window handle without making the field one', async () => {
		await renderField();
		const input = screen.getByRole('textbox', { name: 'Search' });

		expect(input.parentElement).toHaveAttribute('data-tauri-drag-region');
		expect(input).not.toHaveAttribute('data-tauri-drag-region');
	});

	describe('dragging the window by the field', () => {
		beforeEach(() => {
			startDragging.mockClear();
		});

		/** Press, move past the slop, and the OS takes over. */
		it('drags the window when the pointer moves off the press', async () => {
			await renderField();
			const input = screen.getByRole('textbox', { name: 'Search' });

			fireEvent.pointerDown(input, { button: 0, clientX: 100, clientY: 10 });
			fireEvent.pointerMove(window, { clientX: 140, clientY: 10 });

			expect(startDragging).toHaveBeenCalledTimes(1);
			expect(input).not.toHaveFocus();
		});

		/**
		 * The other half of the same gesture. `preventDefault` withholds focus on
		 * the way down so the press can still become a drag, so something has to
		 * give it back when it does not — without this the field can never be
		 * focused by clicking it at all.
		 */
		it('focuses the field when the pointer is released without moving', async () => {
			await renderField();
			const input = screen.getByRole('textbox', { name: 'Search' });

			fireEvent.pointerDown(input, { button: 0, clientX: 100, clientY: 10 });
			fireEvent.pointerUp(window, { clientX: 100, clientY: 10 });

			expect(startDragging).not.toHaveBeenCalled();
			expect(input).toHaveFocus();
		});

		/** A hand is never perfectly still; a 1px wobble is a click, not a drag. */
		it('treats movement inside the slop as a click', async () => {
			await renderField();
			const input = screen.getByRole('textbox', { name: 'Search' });

			fireEvent.pointerDown(input, { button: 0, clientX: 100, clientY: 10 });
			fireEvent.pointerMove(window, { clientX: 102, clientY: 11 });
			fireEvent.pointerUp(window, { clientX: 102, clientY: 11 });

			expect(startDragging).not.toHaveBeenCalled();
			expect(input).toHaveFocus();
		});

		/**
		 * The exception that matters. A field in use has to keep drag-to-select —
		 * miss this and dragging across a query to select it throws the window
		 * across the desktop instead.
		 */
		it('leaves the gesture alone once the field has focus, so text can be selected', async () => {
			await renderField();
			const input = screen.getByRole('textbox', { name: 'Search' });
			input.focus();

			fireEvent.pointerDown(input, { button: 0, clientX: 100, clientY: 10 });
			fireEvent.pointerMove(window, { clientX: 180, clientY: 10 });

			expect(startDragging).not.toHaveBeenCalled();
			expect(input).toHaveFocus();
		});

		/** A right-click opens a context menu; it does not move the window. */
		it('ignores a non-primary button', async () => {
			await renderField();
			const input = screen.getByRole('textbox', { name: 'Search' });

			fireEvent.pointerDown(input, { button: 2, clientX: 100, clientY: 10 });
			fireEvent.pointerMove(window, { clientX: 180, clientY: 10 });

			expect(startDragging).not.toHaveBeenCalled();
		});
	});
});
