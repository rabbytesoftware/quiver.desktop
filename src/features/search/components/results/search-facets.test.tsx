import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { SearchEntry } from '@/domain/search';
import { NO_SELECTION, type Selection } from '@/features/search/lib/narrow';

import { SearchFacets } from './search-facets';

function entry(namespace: string, tags: string[]): SearchEntry {
	return {
		namespace,
		name: namespace.split('/').pop() ?? namespace,
		description: '',
		tags,
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

const ENTRIES = [
	entry('github.com/a/one', ['server', 'java']),
	entry('github.com/b/two', ['server', 'proxy']),
	entry('github.com/c/three', ['server']),
	entry('codeberg.org/d/four', ['proxy']),
];

const NOOP = () => {};

function renderFacets(
	props: { entries?: SearchEntry[]; selection?: Selection; onToggle?: typeof NOOP; onClear?: typeof NOOP } = {}
) {
	return render(
		<SearchFacets
			entries={props.entries ?? ENTRIES}
			onClear={props.onClear ?? NOOP}
			onToggle={props.onToggle ?? NOOP}
			selection={props.selection ?? NO_SELECTION}
		/>
	);
}

/** The row lives in the header, so "renders nothing" has to mean nothing at all. */
describe('SearchFacets', () => {
	it('stays out of the header until there is enough to narrow', () => {
		// Three results are readable as they stand; a row of controls over them is
		// chrome that costs a header row and narrows nothing worth narrowing.
		const { container } = renderFacets({ entries: ENTRIES.slice(0, 3) });
		expect(container).toBeEmptyDOMElement();
	});

	it('names the group, because a bare run of toggles does not say what it does', () => {
		renderFacets();
		// Each chip is a valid toggle on its own, which is why no linter flags the
		// row. Without this a screen reader reaches eight unexplained buttons.
		expect(screen.getByRole('group', { name: 'Narrow results' })).toBeInTheDocument();
	});

	it('names each chip with its value and how many results carry it', () => {
		renderFacets();
		// The visible text is the value and a bare number next to it -- "github.com
		// 3" read aloud is not a sentence.
		expect(screen.getByRole('button', { name: 'github.com, 3 results' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'codeberg.org, 1 result' })).toBeInTheDocument();
	});

	it('offers hosts and tags in one row, counted off the results', () => {
		renderFacets();
		const group = screen.getByRole('group', { name: 'Narrow results' });
		const names = within(group)
			.getAllByRole('button')
			.map((button) => button.getAttribute('aria-label'));

		expect(names).toContain('github.com, 3 results');
		expect(names).toContain('proxy, 2 results');
		// `server` rides on all but one result, so it is the query restated.
		expect(names).not.toContain('server, 3 results');
	});

	it('shows which chips are holding the list down', () => {
		renderFacets({ selection: { host: ['github.com'], tag: [] } });

		expect(screen.getByRole('button', { name: 'github.com, 3 results' })).toHaveAttribute('aria-pressed', 'true');
		expect(screen.getByRole('button', { name: 'codeberg.org, 1 result' })).toHaveAttribute('aria-pressed', 'false');
	});

	it('reports the kind alongside the value, so host and tag cannot collide', async () => {
		const user = userEvent.setup();
		const onToggle = vi.fn();
		renderFacets({ onToggle });

		await user.click(screen.getByRole('button', { name: 'proxy, 2 results' }));

		expect(onToggle).toHaveBeenCalledWith('tag', 'proxy');
	});

	it('counts off the whole answer, not the narrowed one', () => {
		// A facet that vanishes when you select it cannot be unselected, so the
		// caller passes the unnarrowed list and the counts stay put.
		renderFacets({ selection: { host: ['codeberg.org'], tag: [] } });

		expect(screen.getByRole('button', { name: 'github.com, 3 results' })).toBeInTheDocument();
	});

	describe('clear', () => {
		it('is absent until something is actually holding the list down', () => {
			renderFacets();
			expect(screen.queryByRole('button', { name: 'Clear' })).not.toBeInTheDocument();
		});

		it('sits outside the scroller, so it cannot scroll out of reach', () => {
			renderFacets({ selection: { host: ['github.com'], tag: [] } });

			const clear = screen.getByRole('button', { name: 'Clear' });
			const group = screen.getByRole('group', { name: 'Narrow results' });

			expect(clear).toBeInTheDocument();
			expect(group).not.toContainElement(clear);
		});

		it('drops the whole selection at once', async () => {
			const user = userEvent.setup();
			const onClear = vi.fn();
			renderFacets({ onClear, selection: { host: ['github.com'], tag: ['proxy'] } });

			await user.click(screen.getByRole('button', { name: 'Clear' }));

			expect(onClear).toHaveBeenCalledTimes(1);
		});
	});
});
