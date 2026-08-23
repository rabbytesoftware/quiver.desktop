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

function entry(namespace: string, held = false): SearchEntry {
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
		installed: held,
		known: held,
		stars: 0,
		source: null,
	};
}

// ArrowCard renders a router Link, so ResultGrid needs a real router in scope --
// mirrors the harness in arrow-card.test.tsx.
async function renderGrid(props: {
	local: SearchEntry[];
	streamed: SearchEntry[];
	phase: SearchPhase;
	total?: number;
}) {
	const total = props.total ?? props.local.length + props.streamed.length;
	const rootRoute = createRootRoute({
		component: () => <ResultGrid {...props} phase={props.phase} total={total} />,
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

	it('labels the two shelves rather than drawing a seam between the lanes', async () => {
		await renderGrid({ local: [entry('a/one', true)], phase: 'discovering', streamed: [entry('b/two')] });
		expect(screen.getByText('In your vault')).toBeInTheDocument();
		expect(screen.getByText('From the network')).toBeInTheDocument();
	});

	it('keeps the shelves after the settle, when the seam used to dissolve', async () => {
		// Spec 9.3: the settle replaces both bands with one ranked list, so the
		// split has to survive on the entry rather than on the lane it arrived in.
		await renderGrid({ local: [entry('a/one', true), entry('b/two')], phase: 'settled', streamed: [] });
		expect(screen.getByText('In your vault')).toBeInTheDocument();
		expect(screen.getByText('From the network')).toBeInTheDocument();
	});

	it('shows only the shelf that has something in it', async () => {
		await renderGrid({ local: [entry('a/one')], phase: 'settled', streamed: [] });
		expect(screen.queryByText('In your vault')).not.toBeInTheDocument();
		expect(screen.getByText('From the network')).toBeInTheDocument();
	});

	it('shows no skeletons once the pass is over', async () => {
		await renderGrid({ local: [entry('a/one')], phase: 'settled', streamed: [] });
		expect(document.querySelector('[data-slot="card-skeleton"]')).not.toBeInTheDocument();
	});

	it('files a streamed arrow the catalog already holds under the vault, not the network', async () => {
		// The lane is not the question. Discovery reports `installed: true` for an
		// arrow the catalog holds, and Lane A may have ranked it below the limit,
		// so the streamed band is not "things you lack".
		await renderGrid({
			local: [entry('local/unheld')],
			phase: 'discovering',
			streamed: [entry('net/held', true)],
		});
		// TanStack Router carries a splat verbatim -- slashes are not percent-encoded.
		const links = screen.getAllByRole('link').map((a) => a.getAttribute('href'));
		expect(links[0]).toContain('net/held');
		expect(links[1]).toContain('local/unheld');
	});

	it('keeps the ranked lane above the unranked one inside a shelf', async () => {
		await renderGrid({
			local: [entry('local/one')],
			phase: 'discovering',
			streamed: [entry('net/first'), entry('net/second')],
		});
		const links = screen.getAllByRole('link').map((a) => a.getAttribute('href'));
		expect(links[0]).toContain('local/one');
		expect(links[1]).toContain('net/first');
		expect(links[2]).toContain('net/second');
	});

	it('holds the column rule against the whole answer while narrowing', async () => {
		// Two cards shown out of an eight-result answer: the grid keeps the cap the
		// answer earns, so selecting a facet does not resize every tile under the
		// cursor. The rule itself is covered in columns.test.ts -- engines rewrite
		// calc() when serialising, so it is not assertable off the DOM.
		await renderGrid({ local: [entry('a/one'), entry('a/two')], phase: 'settled', streamed: [], total: 8 });
		const wide = document.querySelector('section > div[style*="grid-template-columns"]');
		const wideRule = wide?.getAttribute('style');

		document.body.innerHTML = '';
		await renderGrid({ local: [entry('a/one'), entry('a/two')], phase: 'settled', streamed: [] });
		const thin = document.querySelector('section > div[style*="grid-template-columns"]');

		expect(wideRule).not.toEqual(thin?.getAttribute('style'));
	});

	it('exposes every card as a link, none of them as anything else', async () => {
		const local = [entry('local/one'), entry('local/two')];
		const streamed = [entry('net/first')];
		await renderGrid({ local, phase: 'discovering', streamed });
		expect(screen.getAllByRole('link')).toHaveLength(local.length + streamed.length);
	});
});
