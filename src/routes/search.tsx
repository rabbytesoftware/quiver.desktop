import { createFileRoute } from '@tanstack/react-router';

export interface SearchParams {
	q: string;
}

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
