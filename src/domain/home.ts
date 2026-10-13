import type { SearchEntry } from './search';

export interface HomeShelf {
	id: string;
	/** Shown verbatim. A shelf is never labelled with a host name. */
	title: string;
	/** Null for a shelf that has never been filled. */
	refreshedAt: string | null;
	arrows: SearchEntry[];
}

export interface Home {
	shelves: HomeShelf[];
	/** A refresh is in flight, so an empty snapshot may still fill. */
	refreshing: boolean;
}
