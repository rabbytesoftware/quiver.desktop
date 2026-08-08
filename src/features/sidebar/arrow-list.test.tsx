import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
	useParams,
} from '@tanstack/react-router';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';

import { ArrowList } from './components/arrow-list';

const MINECRAFT = 'github.com/rabbyte/minecraft@v1.21.4';

function entry(namespace: string, name: string, icon: string | null): ArrowEntry {
	return {
		namespace,
		name,
		description: '',
		tags: [],
		icon,
		banner: null,
		version: 'v1.21.4',
		state: 'absent',
		active_run: null,
		last_return: null,
	};
}

/** Seeds the projection the way the catalog stream would, in the order given. */
function seed(...arrows: ArrowEntry[]) {
	useArrowStore.setState({ arrows: new Map(arrows.map((arrow) => [arrow.namespace, arrow])) });
}

/**
 * The list sits in the ROOT component, which is where the rail puts it — above
 * the `<Outlet/>`. A list rendered under a leaf route re-renders on every
 * navigation, which would hide a row that failed to pick up the router's active
 * marking on its own.
 */
function renderList(path: string) {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				<ArrowList />
				<Outlet />
			</>
		),
	});
	const indexRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/',
		component: () => <div data-testid="home" />,
	});
	const arrowRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/arrow/$',
		component: ArrowPage,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, arrowRoute]),
		history: createMemoryHistory({ initialEntries: [path] }),
	});

	render(<RouterProvider router={router} />);
	return router;
}

/** Mirrors `src/routes/arrow.$.tsx`: the namespace on the element, not in the body. */
function ArrowPage() {
	const params = useParams({ strict: false });
	return <div data-testid="arrow-page" data-namespace={params._splat} />;
}

/**
 * Selects an arrow the way the rail does — through the router, with the
 * namespace as a param.
 *
 * Not `initialEntries: ['/arrow/' + namespace]`: the router percent-encodes the
 * `@` when it builds a location, so a hand-written raw one is a different
 * string that matches no link and lights no row. That is an artefact of writing
 * the URL by hand, and asserting against it would be asserting against the
 * test's own typo.
 */
async function selectArrow(router: ReturnType<typeof renderList>, namespace: string) {
	await act(async () => {
		await router.navigate({ to: '/arrow/$', params: { _splat: namespace } });
	});
	await screen.findByTestId('arrow-page');
}

function rows(): HTMLElement[] {
	return screen.getAllByRole('link');
}

function nameOf(row: HTMLElement): string {
	return row.querySelector('[data-slot="arrow-name"]')?.textContent ?? '';
}

function subtitleOf(row: HTMLElement): HTMLElement {
	const subtitle = row.querySelector<HTMLElement>('[data-slot="arrow-namespace"]');
	if (!subtitle) throw new Error('the row has no namespace subtitle');
	return subtitle;
}

beforeEach(() => {
	useArrowStore.getState().reset();
	useLocaleStore.setState({ preference: 'system', detected: 'en' });
	localStorage.removeItem(LOCALE_STORAGE_KEY);
});

describe('ArrowList', () => {
	it('names the list for screen readers', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');
		expect(await screen.findByRole('navigation', { name: 'Arrows' })).toBeInTheDocument();
	});

	/**
	 * The store hands back a `Map`, whose order is insertion order — which is to
	 * say whatever the catalog stream happened to do. These three names are also
	 * chosen so a naive `a.name < b.name` produces exactly the insertion order
	 * back: `<` compares code units, where `Z` (90) precedes `a` (97) and both
	 * precede `É` (201).
	 */
	it('sorts by name rather than by the order the catalog arrived in', async () => {
		seed(
			entry('github.com/rabbyte/zebra@v1', 'Zebra', null),
			entry('github.com/rabbyte/apple@v1', 'apple', null),
			entry('github.com/rabbyte/emile@v1', 'Émile', null)
		);
		renderList('/');

		await screen.findByRole('navigation', { name: 'Arrows' });
		expect(rows().map(nameOf)).toEqual(['apple', 'Émile', 'Zebra']);
	});

	it('renders nothing but the list when the catalog is empty', async () => {
		renderList('/');
		expect(await screen.findByRole('navigation', { name: 'Arrows' })).toBeEmptyDOMElement();
	});
});

describe('ArrowIcon', () => {
	it('shows the arrow’s own icon when it has one, unnamed', async () => {
		seed(entry(MINECRAFT, 'Minecraft', 'https://example.test/mc.png'));
		renderList('/');

		const icon = (await screen.findByRole('link')).querySelector('[data-slot="arrow-icon"]');
		expect(icon).toHaveAttribute('src', 'https://example.test/mc.png');
		// The name is the next thing in the row; a named image says it twice.
		expect(icon).toHaveAttribute('alt', '');
	});

	/**
	 * The design draws no fallback, so this tile is ours. Two lettered tiles are
	 * indistinguishable to a screen reader without the name in the label — the
	 * letter is not the name, and `alt=""` would leave the row's icon unnamed.
	 */
	it('falls back to a named lettered tile when the arrow ships no icon', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const tile = await screen.findByRole('img', { name: 'Minecraft icon' });
		expect(tile).toHaveTextContent('M');
		expect(tile.className).toContain('size-(--icon)');
	});

	/**
	 * `charAt(0)` on a name that opens with an astral character returns half a
	 * surrogate pair, which paints as a replacement glyph rather than the emoji.
	 */
	it('takes a whole code point as the initial, not half a surrogate pair', async () => {
		seed(entry(MINECRAFT, '🐇 Rabbyte', null));
		renderList('/');

		expect(await screen.findByRole('img', { name: '🐇 Rabbyte icon' })).toHaveTextContent('🐇');
	});
});

describe('ArrowRow', () => {
	it('carries the namespace through the splat verbatim', async () => {
		const user = userEvent.setup();
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		await user.click(await screen.findByRole('link'));

		expect(await screen.findByTestId('arrow-page')).toHaveAttribute('data-namespace', MINECRAFT);
	});

	it('is marked active by the router on its own route, and only there', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null), entry('github.com/rabbyte/other@v1', 'Other', null));
		await selectArrow(renderList('/'), MINECRAFT);

		const active = rows().filter((row) => row.getAttribute('data-status') === 'active');
		expect(active).toHaveLength(1);
		expect(nameOf(active[0])).toBe('Minecraft');
	});

	/**
	 * The subtitle is revealed by CSS off the router's own marking, not by a
	 * React branch — it has to reflow live while the rail is being dragged, and
	 * a second copy of "is this row selected" in component state is exactly what
	 * §5.1 removed. jsdom loads no stylesheet, so what is assertable here is the
	 * pair of rules that does it.
	 */
	it('hides the namespace subtitle until the router marks the row active', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		await screen.findByTestId('home');
		const row = rows()[0];
		expect(row).not.toHaveAttribute('data-status', 'active');
		expect(subtitleOf(row).className).toContain('hidden');
		expect(subtitleOf(row).className).toContain('group-data-[status=active]:flex');
	});

	it('shows the whole namespace, split so the version cannot be truncated', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		await selectArrow(renderList('/'), MINECRAFT);

		const subtitle = subtitleOf(rows()[0]);
		expect(subtitle).toHaveTextContent('github.com/rabbyte/minecraft@v1.21.4');

		const [head, tail] = Array.from(subtitle.children);
		expect(head).toHaveTextContent('github.com/rabbyte/minecraft');
		expect(head.className).toContain('truncate');
		expect(tail).toHaveTextContent('@v1.21.4');
		expect(tail.className).toContain('shrink-0');
	});

	/**
	 * The row is one `--row` tall whether or not the subtitle is showing: 13/1.25
	 * plus 10/1.25 is 28.75 inside 34. A height that grew with the subtitle would
	 * shove every row below it down on selection, and jsdom cannot see that
	 * happen — only that the height stopped being a constant.
	 */
	it('keeps the same height class whether or not the subtitle is showing', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		const router = renderList('/');
		await screen.findByTestId('home');
		const inactive = rows()[0].className;

		await selectArrow(router, MINECRAFT);
		const active = rows()[0].className;

		expect(inactive).toContain('h-(--row)');
		// The only class the row gains on selection is the router's own marker.
		// Anything else here would be a second geometry, applied conditionally.
		const added = active.split(' ').filter((name) => !inactive.split(' ').includes(name));
		expect(added).toEqual(['active']);
	});

	/**
	 * `size-[34px]` and `p-[7px]` both LOOK right today and stop tracking `--row`
	 * and `--inset` the moment either moves, with no layout assertion possible in
	 * jsdom to notice.
	 */
	it('sizes itself from the geometry tokens rather than from pixels', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const row = await screen.findByRole('link');
		expect(row.className).toContain('h-(--row)');
		expect(row.className).toContain('p-(--inset)');
		expect(row.className).toContain('gap-(--inset)');
	});

	/**
	 * Excluded rather than overridden: an override still paints the hover fill
	 * for the frame before it wins, so the active row flickers as the cursor
	 * crosses it. Same rule as the history buttons.
	 */
	it('excludes the active row from the hover rule rather than overriding it', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const row = await screen.findByRole('link');
		expect(row.className).toContain('not-data-[status=active]:hover:bg-sidebar-accent');
	});
});
