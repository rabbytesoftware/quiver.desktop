import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';
import type { SearchPhase } from '@/lib/core-store/store/search';

import { ResultGrid } from './result-grid';

function entry(namespace: string): SearchEntry {
	return {
		namespace,
		name: namespace.split('/').pop() ?? namespace,
		description: '',
		tags: [],
		icon: null,
		banner: null,
		versions: ['v1'],
		compatible_os: [],
		provenance: null,
		installed: false,
		known: false,
		stars: 0,
		source: null,
	};
}

// ArrowCard renders a router Link, so ResultGrid needs a real router in scope --
// mirrors the harness in arrow-card.test.tsx.
async function renderGrid(props: { local: SearchEntry[]; streamed: SearchEntry[]; phase: SearchPhase }) {
	const rootRoute = createRootRoute({
		component: () => <ResultGrid {...props} />,
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

	const view = render(<RouterProvider router={router} />);
	await screen.findAllByRole('link');
	return view;
}

describe('ResultGrid', () => {
	it('reserves a row the moment a pass starts, before any result exists', async () => {
		await renderGrid({ local: [entry('a/one')], phase: 'discovering', streamed: [] });
		expect(document.querySelectorAll('[data-slot="card-skeleton"]').length).toBeGreaterThan(0);
	});

	it('separates the streamed band from the local one while a pass runs', async () => {
		await renderGrid({ local: [entry('a/one')], phase: 'discovering', streamed: [entry('b/two')] });
		expect(document.querySelector('[data-slot="results-seam"]')).toBeInTheDocument();
	});

	it('drops the seam once the re-query has settled everything into one order', async () => {
		await renderGrid({ local: [entry('a/one'), entry('b/two')], phase: 'settled', streamed: [] });
		expect(document.querySelector('[data-slot="results-seam"]')).not.toBeInTheDocument();
	});

	it('shows no skeletons once the pass is over', async () => {
		await renderGrid({ local: [entry('a/one')], phase: 'settled', streamed: [] });
		expect(document.querySelector('[data-slot="card-skeleton"]')).not.toBeInTheDocument();
	});

	it('puts streamed results above local ones, in arrival order', async () => {
		await renderGrid({
			local: [entry('local/one')],
			phase: 'discovering',
			streamed: [entry('net/first'), entry('net/second')],
		});
		// TanStack Router carries a splat verbatim -- slashes are not percent-encoded.
		const links = screen.getAllByRole('link').map((a) => a.getAttribute('href'));
		expect(links[0]).toContain('net/first');
		expect(links[1]).toContain('net/second');
		expect(links[2]).toContain('local/one');
	});

	it('exposes every card as a link, none of them as anything else', async () => {
		const local = [entry('local/one'), entry('local/two')];
		const streamed = [entry('net/first')];
		await renderGrid({ local, phase: 'discovering', streamed });
		expect(screen.getAllByRole('link')).toHaveLength(local.length + streamed.length);
	});
});
