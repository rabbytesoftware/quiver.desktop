import type { SearchEntry } from '@/domain/search';
import { isHeld } from '@/domain/search';

const SEPARATORS = /[.\-_/\s]+/g;

function squash(text: string): string {
	return text.toLowerCase().replace(SEPARATORS, '');
}

function tokens(query: string): string[] {
	return query
		.toLowerCase()
		.split(SEPARATORS)
		.filter((token) => token !== '');
}

/**
 * Whether every word of the query appears in what names the arrow. Separators
 * are ignored on both sides, so `quiver.chat`, `quiver-chat` and `quiver chat`
 * all find `quiver.chat`.
 *
 * Descriptions are left out on purpose: core's text index matches them, and
 * that is how a followed collection answered "Quiver.chat" with every arrow
 * whose blurb mentions Quiver.
 */
export function matchesQuery(entry: SearchEntry, query: string): boolean {
	const words = tokens(query);
	if (words.length === 0) return true;
	const haystack = squash([entry.namespace, entry.name, ...entry.tags].join(' '));
	return words.every((word) => haystack.includes(word));
}

/**
 * The network shelf is core's own ranking and passes untouched. The vault shelf
 * is only what the query names: core matches loosely there, and a followed
 * collection would otherwise fill the screen with arrows nobody asked for.
 */
export function isRelevant(entry: SearchEntry, query: string): boolean {
	return !isHeld(entry) || matchesQuery(entry, query);
}
