import { useEffect, useRef } from 'react';

import type { SearchController } from '@/lib/core-store/search/pass';
import { createSearchController } from '@/lib/core-store/search/pass';

/**
 * Binds one controller to the committed URL query. The controller outlives
 * renders but not the route: unmounting closes the socket, which is the cancel
 * (spec 2.4.1).
 */
export function useSearch(query: string): { submit: () => void } {
	const controller = useRef<SearchController | null>(null);

	if (controller.current === null) controller.current = createSearchController();

	useEffect(() => {
		const current = controller.current;
		return () => {
			current?.dispose();
			controller.current = null;
		};
	}, []);

	useEffect(() => {
		controller.current?.setQuery(query);
	}, [query]);

	return { submit: () => controller.current?.submit() };
}
