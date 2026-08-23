import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { sortEntries } from './sort';

function entry(name: string, stars: number): SearchEntry {
	return {
		namespace: `github.com/x/${name}`,
		name,
		description: '',
		tags: [],
		icon: null,
		banner: null,
		versions: [],
		compatible_os: [],
		provenance: null,
		installed: false,
		known: false,
		stars,
		source: null,
	};
}

const ENTRIES = [entry('Velocity', 12), entry('Paper', 900), entry('geyser', 40)];

describe('sortEntries', () => {
	it('leaves relevance alone -- it is the order core ranked, not one we can rebuild', () => {
		expect(sortEntries(ENTRIES, 'relevance', 'en').map((e) => e.name)).toEqual(['Velocity', 'Paper', 'geyser']);
	});

	it('sorts by name in the reader locale, not by code point', () => {
		expect(sortEntries(ENTRIES, 'name', 'en').map((e) => e.name)).toEqual(['geyser', 'Paper', 'Velocity']);
	});

	it('sorts stars descending', () => {
		expect(sortEntries(ENTRIES, 'stars', 'en').map((e) => e.stars)).toEqual([900, 40, 12]);
	});

	it('does not mutate the array it was given', () => {
		const input = [...ENTRIES];
		sortEntries(input, 'stars', 'en');
		expect(input.map((e) => e.name)).toEqual(['Velocity', 'Paper', 'geyser']);
	});
});
