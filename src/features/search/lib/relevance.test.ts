import { describe, expect, it } from 'vitest';

import type { SearchEntry } from '@/domain/search';

import { matchesQuery, relevantLocal } from './relevance';

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

const chat = entry({
	namespace: 'github.com/rabbytesoftware/quiver.chat',
	name: 'Quiver Chat',
	provenance: 'collection',
});
const other = entry({
	namespace: 'github.com/rabbytesoftware/quiver.essentials',
	name: 'Essentials',
	description: 'Works with Quiver chat too',
	provenance: 'collection',
});

describe('matchesQuery', () => {
	it.each([
		'quiver.chat',
		'quiver-chat',
		'quiver_chat',
		'Quiver Chat',
		'quiverchat',
		'quiver/chat',
		' quiver . chat ',
	])('finds quiver.chat by %s', (q) => {
		expect(matchesQuery(chat, q)).toBe(true);
	});

	it.each([
		['quiver_chat', 'quiver.chat'],
		['quiver-chat', 'quiver_chat'],
		['Quiver Chat', 'quiver-chat'],
	])('ignores separators on the entry side too: %s is found by %s', (name, q) => {
		const e = entry({ namespace: 'github.com/x/other', name });
		expect(matchesQuery(e, q)).toBe(true);
	});

	it('does not match on the description', () => {
		expect(matchesQuery(other, 'quiver.chat')).toBe(false);
	});

	it('matches everything for a blank query', () => {
		expect(matchesQuery(other, '  ')).toBe(true);
	});
});

describe('relevantLocal', () => {
	it('keeps only the local entries the query names', () => {
		expect(relevantLocal([chat, other], 'quiver.chat')).toEqual([chat]);
	});
});
