import { useEffect, useRef } from 'react';

import type { SearchController } from '@/lib/core-store/search/pass';
import { createSearchController } from '@/lib/core-store/search/pass';

/**
 * Binds one controller to the committed URL query. The controller outlives
 * renders but not the route: unmounting closes the socket, which is the cancel
 * (spec 2.4.1).
 *
 * The controller is built inside the empty-deps effect, not lazily during
 * render: StrictMode's mount simulation runs setup -> cleanup -> setup again
 * without a re-render, so a ref built during render would have its second
 * setup pass read back a ref the cleanup had just nulled -- see React's
 * "connecting to an external system" guidance. Declared before the `[query]`
 * effect, so the controller exists before the first `setQuery` on every pass.
 */
export function useSearch(query: string): { submit: () => void } {
	const controller = useRef<SearchController | null>(null);

	useEffect(() => {
		controller.current = createSearchController();
		return () => {
			controller.current?.dispose();
			controller.current = null;
		};
	}, []);

	useEffect(() => {
		controller.current?.setQuery(query);
	}, [query]);

	return { submit: () => controller.current?.submit() };
}
