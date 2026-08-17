import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
	useParams,
} from '@tanstack/react-router';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import { LOCALE_STORAGE_KEY, useLocaleStore } from '@/lib/i18n';

import { ArrowList } from './components/arrow-list';
import { ROW_BASE } from './row-base';

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

function seed(...arrows: ArrowEntry[]) {
	useArrowStore.setState({ arrows: new Map(arrows.map((arrow) => [arrow.namespace, arrow])) });
}

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
	const settingsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: '/settings',
		component: () => <div data-testid="settings" />,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([indexRoute, arrowRoute, settingsRoute]),
		history: createMemoryHistory({ initialEntries: [path] }),
	});

	render(<RouterProvider router={router} />);
	return router;
}

function ArrowPage() {
	const params = useParams({ strict: false });
	return <div data-testid="arrow-page" data-namespace={params._splat} />;
}

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

afterEach(() => {
	vi.unstubAllGlobals();
});

function subtitleOf(row: HTMLElement): HTMLElement {
	const subtitle = row.querySelector<HTMLElement>('[data-slot="arrow-namespace"]');
	if (!subtitle) throw new Error('the row has no namespace subtitle');
	return subtitle;
}

function resolveImagesImmediately(): void {
	class LoadedImage {
		onload: (() => void) | null = null;
		onerror: (() => void) | null = null;
		referrerPolicy = '';
		crossOrigin: string | null = null;

		set src(_value: string) {
			queueMicrotask(() => this.onload?.());
		}
	}

	vi.stubGlobal('Image', LoadedImage);
}

async function colourFor(namespace: string, name: string): Promise<string> {
	seed(entry(namespace, name, null));
	renderList('/');
	return (await screen.findByRole('img', { name: `${name} icon` })).style.backgroundColor;
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

	it('shows skeleton rows while the catalog is still loading', async () => {
		useArrowStore.setState({ arrows: new Map(), catalog: 'loading' });
		renderList('/');

		const busy = await screen.findByRole('status', { name: 'Loading arrows' });
		expect(busy).toHaveAttribute('aria-busy', 'true');
		expect(screen.queryByText('No arrows yet')).toBeNull();
	});

	it('says the library is empty once the catalog has actually arrived', async () => {
		useArrowStore.setState({ arrows: new Map(), catalog: 'ready' });
		renderList('/');

		expect(await screen.findByText('No arrows yet')).toBeVisible();
		expect(screen.queryByRole('status')).toBeNull();
	});

	// The distinction this guards is the whole point: a rail that renders the
	// same blank space for "you have no arrows" and "the daemon is unreachable"
	// tells the user their library is gone when it is only out of reach.
	it('distinguishes an unreachable core from an empty library', async () => {
		useArrowStore.setState({ arrows: new Map(), catalog: 'error' });
		renderList('/');

		expect(await screen.findByText('Can’t reach quiver.core')).toBeVisible();
		expect(screen.queryByText('No arrows yet')).toBeNull();
		expect(screen.getByRole('link', { name: 'Check Engine settings' })).toHaveAttribute(
			'href',
			'/settings?tab=engine'
		);
	});

	it('prefers cached arrows over any placeholder, even mid-load', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		useArrowStore.setState({ catalog: 'loading' });
		renderList('/');

		await screen.findByRole('navigation', { name: 'Arrows' });
		expect(rows().map(nameOf)).toEqual(['Minecraft']);
		expect(screen.queryByRole('status')).toBeNull();
	});
});

describe('ArrowIcon', () => {
	it('shows the arrow’s own icon when it has one, unnamed', async () => {
		resolveImagesImmediately();
		seed(entry(MINECRAFT, 'Minecraft', 'https://example.test/mc.png'));
		renderList('/');

		const row = await screen.findByRole('link');
		await waitFor(() => expect(row.querySelector('img')).not.toBeNull());

		const icon = row.querySelector('img');
		expect(icon).toHaveAttribute('src', 'https://example.test/mc.png');
		expect(icon).toHaveAttribute('alt', '');
	});

	it('keeps the monogram when the icon URL never resolves', async () => {
		seed(entry(MINECRAFT, 'Minecraft', 'https://example.test/gone.png'));
		renderList('/');

		expect(await screen.findByRole('img', { name: 'Minecraft icon' })).toBeInTheDocument();
		expect((await screen.findByRole('link')).querySelector('img')).toBeNull();
	});

	it('falls back to a named monogram chip when the arrow ships no icon', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const tile = await screen.findByRole('img', { name: 'Minecraft icon' });
		expect(tile.closest('[data-slot="arrow-icon"]')?.className).toContain('size-(--icon)');
		expect(tile.textContent).toBe('Mi');
	});

	it.each([
		['Minecraft Server', 'MS'],
		['Firefox', 'Fi'],
		['Q', 'Q'],
		['\u{1F407} Rabbyte', '\u{1F407}R'],
		['\u{1F407}Rabbyte', '\u{1F407}R'],
	])('takes a two-glyph monogram from %s', async (name, expected) => {
		seed(entry(MINECRAFT, name, null));
		renderList('/');

		const tile = await screen.findByRole('img', { name: `${name} icon` });
		expect(tile.textContent).toBe(expected);
	});

	it('gives a namespace the same colour every time it renders', async () => {
		const first = await colourFor(MINECRAFT, 'Minecraft');
		cleanup();
		const second = await colourFor(MINECRAFT, 'Minecraft');

		expect(second).toBe(first);
	});

	it('gives different namespaces different colours', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null), entry('github.com/mozilla/firefox@v1', 'Firefox', null));
		renderList('/');

		await screen.findByRole('navigation', { name: 'Arrows' });
		const [first, second] = screen.getAllByRole('img').map((tile) => tile.style.backgroundColor);
		expect(first).not.toBe(second);
	});

	it('varies only the hue, at a lightness white text stays legible on', async () => {
		expect(await colourFor(MINECRAFT, 'Minecraft')).toMatch(/^oklch\(0\.52 0\.15 \d{1,3}\)$/);
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

	it('keeps the same height class whether or not the subtitle is showing', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		const router = renderList('/');
		await screen.findByTestId('home');
		const inactive = rows()[0].className;

		await selectArrow(router, MINECRAFT);
		const active = rows()[0].className;

		expect(inactive).toContain('h-9');
		const added = active.split(' ').filter((name) => !inactive.split(' ').includes(name));
		expect(added).toEqual(['active']);
	});

	it('sizes itself from the geometry tokens rather than from pixels', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const row = await screen.findByRole('link');
		expect(row.className).toContain(ROW_BASE);
		expect(row.className).toContain('rounded-lg');
		expect(row.className).not.toMatch(/rounded-\[/);
	});

	it('excludes the active row from the hover rule rather than overriding it', async () => {
		seed(entry(MINECRAFT, 'Minecraft', null));
		renderList('/');

		const row = await screen.findByRole('link');
		expect(row.className).toContain('not-data-[status=active]:hover:bg-accent');
	});
});
