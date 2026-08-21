import { StrictMode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
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
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { IDLE_BEFORE_PASS_MS } from '@/lib/core-store/search/pass';
import { useSearchStore } from '@/lib/core-store/store/search';
import { createMockBackend, type MockRuntime } from '@/lib/mock';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { ResultsScreen } from './results-screen';
import { SearchBar } from './search-bar';

let mock: MockRuntime;

beforeEach(() => {
	mock = createMockBackend('normal');
	installBackend(mock.backend);
	useSearchStore.getState().reset();
});

afterEach(() => {
	mock.dispose();
	resetBackend();
});

// ArrowCard renders a router Link, so ResultsScreen needs a real router in
// scope -- mirrors the harness in result-grid.test.tsx.
function renderScreen(query: string) {
	const rootRoute = createRootRoute({
		component: () => <ResultsScreen query={query} />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	return render(<RouterProvider router={router} />);
}

describe('ResultsScreen', () => {
	it('shows local results without waiting for the network lane', async () => {
		renderScreen('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
		expect(screen.getByRole('heading', { name: 'minecraft' })).toBeInTheDocument();
	});

	it('renders no heading and no results for an empty query', () => {
		renderScreen('');
		expect(screen.queryByRole('heading')).not.toBeInTheDocument();
		expect(screen.queryAllByRole('link')).toHaveLength(0);
	});

	// The resting screen has no heading and no grid, so without this line it is
	// a blank canvas -- the design carried one centred sentence here.
	it('invites the first keystroke when nothing has been asked for', async () => {
		renderScreen('');
		expect(await screen.findByText('Type to search.')).toBeInTheDocument();
	});

	// `setQuery` empties the store before Lane A answers, so a real query sits
	// in `idle` for a moment. Inviting a search there would talk over one that
	// is already happening.
	it('does not invite a search while one is already on its way', async () => {
		renderScreen('minecraft');
		await screen.findByTestId('search-page');
		expect(screen.queryByText('Type to search.')).not.toBeInTheDocument();
	});

	// An empty field over a full grid is the state this screen must never reach.
	// It is only reachable across a remount, because the store outlives the
	// screen and a fresh controller starts out holding the same blank query it
	// is about to be handed.
	it('does not inherit the last screen results when it reopens on a blank query', async () => {
		const first = renderScreen('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
		first.unmount();

		renderScreen('');
		// The router resolves the route a tick after render, so asserting on the
		// DOM straight away would pass against an empty tree and prove nothing.
		await screen.findByTestId('search-page');

		expect(useSearchStore.getState().local).toEqual([]);
		expect(screen.queryAllByRole('link')).toHaveLength(0);
		expect(screen.queryByRole('heading')).not.toBeInTheDocument();
	});

	it('does not go back to the network when it opens on a restored query', async () => {
		vi.useFakeTimers();
		try {
			useSearchStore.getState().requestRestore('server');
			renderScreen('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);

			expect(useSearchStore.getState().local.length).toBeGreaterThan(0);
			expect(useSearchStore.getState().phase).toBe('local');
			expect(useSearchStore.getState().job).toBeNull();
		} finally {
			vi.useRealTimers();
		}
	});

	// Mirrors pass.test.ts's "does not outlive dispose" at the screen level:
	// unmounting is the cancel, so a pass in flight must not keep mutating the
	// store once its screen is gone.
	it('does not let a pass outlive the screen that started it', async () => {
		vi.useFakeTimers();
		try {
			const { unmount } = renderScreen('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);
			expect(useSearchStore.getState().phase).toBe('discovering');

			unmount();
			const before = useSearchStore.getState();
			await vi.advanceTimersByTimeAsync(30_000);
			expect(useSearchStore.getState()).toEqual(before);
		} finally {
			vi.useRealTimers();
		}
	});

	it('carries exactly one aria-live region on the whole screen, on the header meta line', async () => {
		renderScreen('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
		expect(document.querySelectorAll('[aria-live]')).toHaveLength(1);
	});

	// Real-window repro: 'server' both matches local results and hits
	// gitlab.com's fixture refusal, so a settled pass has results and a
	// refusal at once -- the empty state must stay out of the way of both.
	it('does not print the refusal a second time once results have settled onto the screen', async () => {
		vi.useFakeTimers();
		try {
			renderScreen('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 10_000);

			expect(useSearchStore.getState().phase).toBe('settled');
			expect(screen.getAllByRole('link').length).toBeGreaterThan(0);
			expect(screen.getAllByText(/refused/)).toHaveLength(1);
			expect(screen.queryByText(/every host answered/)).not.toBeInTheDocument();
		} finally {
			vi.useRealTimers();
		}
	});
});

// Mirrors spec 9.7/8.9: the FLIP settle re-measures cards after `local`/
// `streamed` change and animates the delta -- unless the OS asked for reduced
// motion, in which case the animation must never run at all.
describe('ResultsScreen card motion', () => {
	afterEach(() => {
		vi.unstubAllGlobals();
		vi.restoreAllMocks();
		delete (Element.prototype as unknown as { animate?: unknown }).animate;
	});

	function stubMotion(reduced: boolean): void {
		vi.stubGlobal(
			'matchMedia',
			vi.fn().mockImplementation((query: string) => ({
				matches: reduced,
				media: query,
				addEventListener: vi.fn(),
				removeEventListener: vi.fn(),
				addListener: vi.fn(),
				removeListener: vi.fn(),
				dispatchEvent: vi.fn(),
			}))
		);
	}

	// jsdom has no layout, so real cards never move; forcing distinct rects on
	// each measurement is the only way to produce a non-zero FLIP delta here.
	function stubMovingRects(): void {
		let call = 0;
		vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function () {
			call += 10;
			return {
				left: call,
				top: 0,
				right: call,
				bottom: 0,
				width: 0,
				height: 0,
				x: call,
				y: 0,
				toJSON: () => ({}),
			} as DOMRect;
		});
	}

	it('animates the settle when a card moves and motion is not reduced', async () => {
		stubMovingRects();
		stubMotion(false);
		const animateSpy = vi.fn();
		Element.prototype.animate = animateSpy as unknown as Element['animate'];

		renderScreen('server');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));

		act(() => {
			useSearchStore.getState().setLocal([...useSearchStore.getState().local].reverse());
		});

		await waitFor(() => expect(animateSpy).toHaveBeenCalled());
	});

	it('never animates the settle when the OS asks for reduced motion, even though a card moved', async () => {
		stubMovingRects();
		stubMotion(true);
		const animateSpy = vi.fn();
		Element.prototype.animate = animateSpy as unknown as Element['animate'];

		renderScreen('server');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));

		act(() => {
			useSearchStore.getState().setLocal([...useSearchStore.getState().local].reverse());
		});

		await Promise.resolve();
		expect(animateSpy).not.toHaveBeenCalled();
	});
});

// Mirrors main.tsx exactly: StrictMode wraps QueryClientProvider wraps
// RouterProvider, all above this screen. StrictMode nested any lower (e.g.
// directly around ResultsScreen, below the router) does not reproduce the
// bug this guards against -- the router's own effects are part of what
// double-invokes at the real root.
function renderAtAppRoot(query: string) {
	const rootRoute = createRootRoute({
		component: () => <ResultsScreen query={query} />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: () => <div data-testid="arrow-page" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([arrowRoute]),
		history: createMemoryHistory({ initialEntries: ['/'] }),
	});

	const queryClient = new QueryClient();

	return render(
		<StrictMode>
			<QueryClientProvider client={queryClient}>
				<RouterProvider router={router} />
			</QueryClientProvider>
		</StrictMode>
	);
}

describe('ResultsScreen mounted at the real app root', () => {
	it('still starts a search under StrictMode, not just in a plain render', async () => {
		renderAtAppRoot('minecraft');
		await waitFor(() => expect(screen.getAllByRole('link').length).toBeGreaterThan(0));
	});

	// StrictMode runs setup, cleanup, setup. A restore request consumed on the
	// first setup leaves the second one looking at an ordinary query, which arms
	// the pass the restore existed to avoid -- invisible in a plain render, and
	// the whole point of the request in the app.
	it('honours a restore on the second setup too, not just the first', async () => {
		vi.useFakeTimers();
		try {
			useSearchStore.getState().requestRestore('server');
			renderAtAppRoot('server');
			await vi.advanceTimersByTimeAsync(IDLE_BEFORE_PASS_MS + 400);

			expect(useSearchStore.getState().phase).toBe('local');
			expect(useSearchStore.getState().job).toBeNull();
		} finally {
			vi.useRealTimers();
		}
	});
});

// SearchBar (the rail field) and ResultsScreen (the route body) are siblings
// under the root route, not parent/child -- this mirrors that shape so Enter's
// wiring is exercised the way the real app renders it, not through a stub.
function renderAppWithField(initialEntries = ['/']) {
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
		component: () => <ResultsScreen query={searchRoute.useSearch().q} />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, searchRoute]),
		history: createMemoryHistory({ initialEntries }),
	});

	return render(<RouterProvider router={router} />);
}

describe('Enter fires the pass immediately (spec 2.2)', () => {
	afterEach(() => vi.useRealTimers());

	it('starts a pass without waiting out the idle timer', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const user = userEvent.setup();

		renderAppWithField();
		const input = await screen.findByRole('textbox', { name: 'Search' });

		await user.type(input, 'server{Enter}');
		await vi.advanceTimersByTimeAsync(10);

		expect(useSearchStore.getState().phase).toBe('discovering');
	});

	// Regression: setQuery and submit used to share one generation counter, so
	// submit's cancel discarded the local fetch setQuery had just started --
	// the grid rendered no local band for the whole pass (spec 2.2.1).
	it('keeps the local band populated through the pass, not just the phase', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const user = userEvent.setup();

		renderAppWithField();
		const input = await screen.findByRole('textbox', { name: 'Search' });

		await user.type(input, 'server{Enter}');
		await vi.advanceTimersByTimeAsync(10);

		expect(useSearchStore.getState().phase).toBe('discovering');
		expect(useSearchStore.getState().local.length).toBeGreaterThan(0);
	});

	it('fires again on Enter even when the committed query is unchanged, rather than doing nothing', async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		const user = userEvent.setup();

		renderAppWithField(['/search?q=server']);
		const input = await screen.findByRole('textbox', { name: 'Search' });
		await vi.advanceTimersByTimeAsync(0);
		expect(useSearchStore.getState().phase).toBe('local');

		await user.click(input);
		await user.keyboard('{Enter}');
		await vi.advanceTimersByTimeAsync(10);

		expect(useSearchStore.getState().phase).toBe('discovering');
	});
});
