import type { SearchEntry } from '@/domain/search';

/**
 * Spec 17 refused a sort control on one argument: with no text on a card at
 * rest, it reorders things the user cannot read. The caption (spec 8.7) is what
 * retires that argument, so this arrived with it and not before.
 *
 * All three act on the answer the client holds. Core exposes no ordering
 * parameter -- `GET /v0/search` takes `q`, `limit` and `os` -- so relevance is
 * the order core returned and the other two are client-side. That is honest
 * only because the client asks for core's cap rather than its default
 * (`SEARCH_LIMIT`, spec 1.1).
 */
export const SORT_KEYS = ['relevance', 'name', 'stars'] as const;

export type SortKey = (typeof SORT_KEYS)[number];

export const DEFAULT_SORT: SortKey = 'relevance';

export function sortEntries(entries: SearchEntry[], sort: SortKey, locale: string): SearchEntry[] {
	if (sort === 'relevance') return entries;
	const sorted = [...entries];
	if (sort === 'name') sorted.sort((a, b) => a.name.localeCompare(b.name, locale));
	// Descending, and ties keep the order core ranked them in.
	if (sort === 'stars') sorted.sort((a, b) => b.stars - a.stars);
	return sorted;
}
