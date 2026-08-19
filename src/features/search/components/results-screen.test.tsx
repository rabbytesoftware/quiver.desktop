import { StrictMode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { IDLE_BEFORE_PASS_MS } from '@/lib/core-store/search/pass';
import { useSearchStore } from '@/lib/core-store/store/search';
import { createMockBackend, type MockRuntime } from '@/lib/mock';
import { installBackend, resetBackend } from '@/lib/transport/backend';

import { ResultsScreen } from './results-screen';

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

	it('renders nothing but the empty frame for an empty query', () => {
		renderScreen('');
		expect(screen.queryByRole('heading')).not.toBeInTheDocument();
		expect(screen.queryAllByRole('link')).toHaveLength(0);
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
});
