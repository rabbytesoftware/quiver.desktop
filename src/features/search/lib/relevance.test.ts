import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { isRelevant, matchesQuery } from './relevance';

function entry(overrides: Partial<SearchEntry>): SearchEntry {
	return {
		namespace: 'github.com/owner/repo',
		name: 'repo',
		description: '',
		tags: [],
		icon: null,
		banner: null,
		versions: [],
		compatible_os: [],
		provenance: null,
		installed: false,
		known: true,
		stars: 0,
		source: null,
		...overrides,
	};
}

const chat = entry({ namespace: 'github.com/rabbytesoftware/quiver.chat', name: 'Quiver Chat', provenance: 'collection' });
const other = entry({
	namespace: 'github.com/rabbytesoftware/quiver.essentials',
	name: 'Essentials',
	description: 'Works with Quiver chat too',
	provenance: 'collection',
});

describe('matchesQuery', () => {
	it.each(['quiver.chat', 'quiver-chat', 'quiver_chat', 'Quiver Chat', 'quiverchat'])('finds quiver.chat by %s', (q) => {
		expect(matchesQuery(chat, q)).toBe(true);
	});

	it('does not match on the description', () => {
		expect(matchesQuery(other, 'quiver.chat')).toBe(false);
	});

	it('matches everything for a blank query', () => {
		expect(matchesQuery(other, '  ')).toBe(true);
	});
});

describe('isRelevant', () => {
	it('filters held entries by the query', () => {
		expect(isRelevant(other, 'quiver.chat')).toBe(false);
		expect(isRelevant(chat, 'quiver.chat')).toBe(true);
	});

	it('never filters the network shelf', () => {
		expect(isRelevant(entry({ namespace: 'github.com/x/y', name: 'unrelated' }), 'quiver.chat')).toBe(true);
	});
});
