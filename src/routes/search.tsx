import { createFileRoute } from '@tanstack/react-router';

import { ResultsScreen } from '@/features/search';

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

	return <ResultsScreen query={q} />;
}
