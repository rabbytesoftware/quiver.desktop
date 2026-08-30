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
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { resolveNamespaceTarget } from '@/features/search/api/resolve-namespace';
import { LOCAL_DEBOUNCE_MS, SearchBar } from '@/features/search';
import { useSearchStore } from '@/lib/core-store/store/search';

vi.mock('@/features/search/api/resolve-namespace', () => ({ resolveNamespaceTarget: vi.fn() }));

const mockResolveNamespaceTarget = vi.mocked(resolveNamespaceTarget);

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
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});
	const collectionRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/collection/$',
		component: () => <div data-testid="collection-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, searchRoute, arrowRoute, collectionRoute]),
		history: createMemoryHistory({ initialEntries }),
	});

	render(<RouterProvider router={router} />);

	const input = await screen.findByRole('textbox', { name: 'Search' });
	const user = userEvent.setup();
	return { input, router, user };
}

describe('SearchBar', () => {
	beforeEach(() => {
		mockResolveNamespaceTarget.mockReset();
		mockResolveNamespaceTarget.mockResolvedValue(null);
	});

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

	// Emptying the field asks for an empty search. It used to pop history, which
	// landed wherever the stack happened to point -- home, an older search, or
	// the arrow page you had just come back from -- and popping onto another
	// /search entry refilled the field with the query you had just deleted.
	//
	// Each of these sleeps past the debounce rather than using `waitFor`: the
	// assertion is that the route does NOT change, and `waitFor` would satisfy
	// itself on the state before the commit had a chance to move it.
	const settle = () => new Promise((resolve) => setTimeout(resolve, LOCAL_DEBOUNCE_MS + 150));

	it('stays on an empty results screen when the query is emptied, rather than popping history', async () => {
		const { input, router, user } = await renderField();

		await user.type(input, 'minecraft');
		await waitFor(() => expect(router.state.location.search).toEqual({ q: 'minecraft' }));
		expect(router.history.length).toBe(2);

		await user.clear(input);
		await settle();

		expect(router.state.location.pathname).toBe('/search');
		expect(router.state.location.search).toEqual({ q: '' });
		expect(input).toHaveValue('');
		expect(router.history.length).toBe(2);
	});

	// The stack that used to bite: popping from here landed on the older search
	// and refilled the field with the query that had just been deleted.
	it('lands on the empty search even when the entry behind it is another search', async () => {
		const { input, router, user } = await renderField(['/', '/search?q=redis', '/search?q=minecraft']);
		expect(input).toHaveValue('minecraft');

		await user.clear(input);
		await settle();

		expect(router.state.location.pathname).toBe('/search');
		expect(router.state.location.search).toEqual({ q: '' });
		expect(input).toHaveValue('');
	});

	// Clearing from another page cannot happen without focusing the field first,
	// and focusing reopens the search it is holding. So the two steps read as
	// one: reopen minecraft, then empty it -- landing on the empty search rather
	// than on whatever history had underneath.
	it('empties into an empty search when reopened from another page and then cleared', async () => {
		const { input, router, user } = await renderField(['/search?q=minecraft']);

		await act(async () => {
			await router.navigate({ to: '/' });
		});
		await screen.findByTestId('home');
		expect(input).toHaveValue('minecraft');

		await user.clear(input);
		await settle();

		expect(router.state.location.pathname).toBe('/search');
		expect(router.state.location.search).toEqual({ q: '' });
		expect(input).toHaveValue('');
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

	// Leaving the results route used to blank the field, which threw the query
	// away and left focusing it again as the only way back -- onto a blank
	// search. The field is in the sidebar and outlives the route; the query it
	// is holding should outlive it too.
	it('keeps the query after leaving /search, rather than throwing it away', async () => {
		const { input, router } = await renderField(['/search?q=minecraft']);
		expect(input).toHaveValue('minecraft');

		await act(async () => {
			await router.navigate({ to: '/' });
		});

		expect(await screen.findByTestId('home')).toBeInTheDocument();
		expect(input).toHaveValue('minecraft');
	});

	it('reopens the search it is still holding when the field is focused again', async () => {
		const { input, router } = await renderField(['/search?q=minecraft']);
		await act(async () => {
			await router.navigate({ to: '/' });
		});
		await screen.findByTestId('home');

		input.focus();
		await screen.findByTestId('search-page');

		expect(router.state.location.pathname).toBe('/search');
		expect(router.state.location.search).toEqual({ q: 'minecraft' });
		expect(input).toHaveValue('minecraft');
	});

	// A deep link is still authoritative: it names a query, and that one wins
	// over whatever the field happened to be holding.
	it('takes the query from a link into /search over the one it was holding', async () => {
		const { input, router } = await renderField(['/search?q=minecraft']);
		await act(async () => {
			await router.navigate({ to: '/' });
		});

		await act(async () => {
			await router.navigate({ to: '/search', search: { q: 'redis' } });
		});

		expect(input).toHaveValue('redis');
	});

	it('asks for a restore, not a fresh pass, when it reopens the search it holds', async () => {
		const { input, router } = await renderField(['/search?q=minecraft']);
		await act(async () => {
			await router.navigate({ to: '/' });
		});
		useSearchStore.getState().clearRestore();

		input.focus();
		await screen.findByTestId('search-page');

		expect(useSearchStore.getState().restoreQuery).toBe('minecraft');
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
		// Enter now awaits a (mocked, here) namespace check before committing --
		// a microtask hop past what `user.type` itself waits out.
		await waitFor(() => expect(router.state.location.search).toEqual({ q: 'serv' }));

		vi.useRealTimers();
	});

	it('does not check for a namespace, or start a search, when Enter is pressed on an empty field', async () => {
		const { input, router, user } = await renderField(['/search?q=']);

		await user.click(input);
		await user.keyboard('{Enter}');

		expect(mockResolveNamespaceTarget).not.toHaveBeenCalled();
		expect(router.state.location.search).toEqual({ q: '' });
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

	describe('Enter redirecting to a resolved namespace', () => {
		it('goes straight to the collection page instead of searching', async () => {
			mockResolveNamespaceTarget.mockResolvedValue({ kind: 'collection', namespace: 'github.com/rabbyte/game-servers' });
			const { input, router, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/game-servers{Enter}');

			expect(await screen.findByTestId('collection-page')).toBeInTheDocument();
			expect(router.state.location.pathname).toBe('/collection/github.com/rabbyte/game-servers');
		});

		it('goes straight to the arrow page instead of searching', async () => {
			mockResolveNamespaceTarget.mockResolvedValue({ kind: 'arrow', namespace: 'github.com/rabbyte/minecraft@v1.21.4' });
			const { input, router, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/minecraft@v1.21.4{Enter}');

			// The router percent-encodes `@` in a splat segment (see
			// collection-arrow-tile.test.tsx for the same rule on the Link side).
			expect(await screen.findByTestId('arrow-page')).toBeInTheDocument();
			expect(router.state.location.pathname).toBe('/arrow/github.com/rabbyte/minecraft%40v1.21.4');
		});

		it('does not also start a search pass for text it redirected on', async () => {
			useSearchStore.getState().clearSubmit();
			mockResolveNamespaceTarget.mockResolvedValue({ kind: 'collection', namespace: 'github.com/rabbyte/game-servers' });
			const { input, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/game-servers{Enter}');

			await screen.findByTestId('collection-page');
			expect(useSearchStore.getState().submitQuery).toBeNull();
		});

		it('searches normally when nothing resolves', async () => {
			mockResolveNamespaceTarget.mockResolvedValue(null);
			const { input, router, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/unknown-thing{Enter}');

			await waitFor(() => expect(router.state.location.search).toEqual({ q: 'github.com/rabbyte/unknown-thing' }));
			expect(router.state.location.pathname).toBe('/search');
		});

		it('searches normally when the namespace check itself fails', async () => {
			mockResolveNamespaceTarget.mockRejectedValue(new Error('backend unreachable'));
			const { input, router, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/game-servers{Enter}');

			await waitFor(() =>
				expect(router.state.location.search).toEqual({ q: 'github.com/rabbyte/game-servers' })
			);
			expect(router.state.location.pathname).toBe('/search');
		});

		it('never checks while merely typing -- only Enter can redirect', async () => {
			mockResolveNamespaceTarget.mockResolvedValue({ kind: 'collection', namespace: 'github.com/rabbyte/game-servers' });
			const { input, router, user } = await renderField();

			await user.type(input, 'github.com/rabbyte/game-servers');

			await waitFor(() => expect(router.state.location.pathname).toBe('/search'));
			expect(mockResolveNamespaceTarget).not.toHaveBeenCalled();
		});
	});
});
