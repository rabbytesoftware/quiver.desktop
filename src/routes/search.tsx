import { createFileRoute } from '@tanstack/react-router';

export interface SearchParams {
	q: string;
}

/**
 * `?q=` is the only place the query lives (spec §1.6), so it arrives as
 * whatever the address bar had. `?q=a&q=b` parses to an array and a bare `?q`
 * to `undefined`; either one reaches the results as a non-string and every
 * `.trim()` and `.toLowerCase()` downstream throws. Anything that is not a
 * string collapses to the empty query.
 */
function validateSearch(search: Record<string, unknown>): SearchParams {
	return { q: typeof search.q === 'string' ? search.q : '' };
}

export const Route = createFileRoute('/search')({
	validateSearch,
	component: SearchPage,
});

function SearchPage() {
	const { q } = Route.useSearch();

	return <div data-testid="search-page">{q}</div>;
}
