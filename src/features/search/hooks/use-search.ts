import { useEffect, useRef } from 'react';

import type { SearchController } from '@/lib/core-store/search/pass';
import { createSearchController } from '@/lib/core-store/search/pass';
import { useSearchStore } from '@/lib/core-store/store/search';

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
export function useSearch(query: string): void {
	const controller = useRef<SearchController | null>(null);
	const submitQuery = useSearchStore((s) => s.submitQuery);

	useEffect(() => {
		controller.current = createSearchController();
		return () => {
			controller.current?.dispose();
			controller.current = null;
		};
	}, []);

	// A restore request names the query it belongs to, so a query typed between
	// the request and this mount is still treated as the new ask it is.
	//
	// It is dropped when it stops matching rather than when it is first read.
	// StrictMode runs setup, cleanup, setup: consuming on the first setup left
	// the second one holding an ordinary query and arming the very pass the
	// request existed to avoid -- which a plain render never shows.
	useEffect(() => {
		const { restoreQuery, clearRestore } = useSearchStore.getState();
		const restoring = restoreQuery === query;
		if (!restoring && restoreQuery !== null) clearRestore();
		controller.current?.setQuery(query, { discover: !restoring });
	}, [query]);

	// `SearchBar` lives outside this route, so Enter (spec 2.2) reaches the
	// controller through the store rather than a prop. Read `submitQuery` via
	// `getState()`, not the subscribed value above, so StrictMode's extra
	// invocation of this same effect (no cleanup to undo it) sees the request
	// already consumed instead of firing the pass twice.
	useEffect(() => {
		const pending = useSearchStore.getState().submitQuery;
		if (pending === null) return;
		useSearchStore.getState().clearSubmit();
		controller.current?.setQuery(pending);
		controller.current?.submit();
	}, [submitQuery]);
}
