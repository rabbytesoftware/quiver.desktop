import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { NO_SELECTION, applySelection, facetsFor, hostOf, isNarrowed, toggle } from './narrow';

function entry(namespace: string, tags: string[]): SearchEntry {
	return {
		namespace,
		name: namespace.split('/').pop() ?? namespace,
		description: '',
		tags,
		icon: null,
		banner: null,
		versions: [],
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

describe('facetsFor', () => {
	it('counts hosts off the results, so a facet cannot name what the list lacks', () => {
		expect(facetsFor(ENTRIES, 'host', 6)).toEqual([
			{ value: 'github.com', count: 3 },
			{ value: 'codeberg.org', count: 1 },
		]);
	});

	it('drops a tag all but one result carries, which is the query restated', () => {
		// `server` is on 3 of 4: selecting it would remove a single card while
		// looking like a filter.
		expect(facetsFor(ENTRIES, 'tag', 8).map((f) => f.value)).toEqual(['proxy', 'java']);
	});

	it('orders by count, then alphabetically, so the list is stable', () => {
		expect(facetsFor(ENTRIES, 'tag', 8)).toEqual([
			{ value: 'proxy', count: 2 },
			{ value: 'java', count: 1 },
		]);
	});

	it('takes at most the number asked for', () => {
		expect(facetsFor(ENTRIES, 'tag', 1)).toHaveLength(1);
	});
});

describe('applySelection', () => {
	it('keeps everything when nothing is selected', () => {
		expect(applySelection(ENTRIES, NO_SELECTION)).toHaveLength(4);
	});

	it('widens inside one facet', () => {
		const selection = { ...NO_SELECTION, tag: ['java', 'proxy'] };
		expect(applySelection(ENTRIES, selection).map(hostOf)).toEqual(['github.com', 'github.com', 'codeberg.org']);
	});

	it('narrows across facets', () => {
		const selection = { host: ['github.com'], tag: ['proxy'] };
		expect(applySelection(ENTRIES, selection).map((e) => e.namespace)).toEqual(['github.com/b/two']);
	});

	it('can narrow to nothing rather than silently ignoring a combination', () => {
		expect(applySelection(ENTRIES, { host: ['codeberg.org'], tag: ['java'] })).toEqual([]);
	});
});

describe('toggle', () => {
	it('adds then removes, leaving the other facet alone', () => {
		const on = toggle(NO_SELECTION, 'host', 'github.com');
		expect(on.host).toEqual(['github.com']);
		expect(on.tag).toEqual([]);
		expect(isNarrowed(on)).toBe(true);

		const off = toggle(on, 'host', 'github.com');
		expect(off.host).toEqual([]);
		expect(isNarrowed(off)).toBe(false);
	});
});
