import { apiFetch } from '@/lib/transport/api';
import { backend, type SocketLike } from '@/lib/transport/backend';

import type { DiscoveryJobDTO, DiscoveryJobStartedDTO, SearchResultDTO } from '../dtos/v0/search';
import { toDiscoverySummary, toSearchEntry } from '../dtos/v0/search';
import { useSearchStore } from '../store/search';

/** Stillness before a provider pass. Measured from the committed query -- spec 2.2.1. */
export const IDLE_BEFORE_PASS_MS = 600;
/** The stream has no terminal frame, so completion is polled -- spec 1.4. */
export const POLL_INTERVAL_MS = 1000;
/** provider_timeout (10s), doubled, plus slack. Spec 1.4.1. */
export const PASS_DEADLINE_MS = 25_000;
/** Core's cap (`maxLimit`), not its default of 25. Spec 1.1. */
export const SEARCH_LIMIT = 100;

export interface SearchQueryOptions {
	/**
	 * Whether this query is worth a provider pass. False when the screen is
	 * being restored rather than asked for -- Lane A still runs, because the
	 * vault holds what the last pass found.
	 */
	discover?: boolean;
}

export interface SearchController {
	/** Called with a committed (URL) query -- already debounced by the field. */
	setQuery: (query: string, options?: SearchQueryOptions) => void;
	/** Enter: fire the pass now. */
	submit: () => void;
	dispose: () => void;
}

/** Neither lane is ever called with this -- core 400s on Lane A, and the mock trims before matching. */
function isBlank(q: string): boolean {
	return q.trim() === '';
}

export function createSearchController(): SearchController {
	const store = useSearchStore;

	let query = '';
	let idleTimer: ReturnType<typeof setTimeout> | null = null;
	let pollTimer: ReturnType<typeof setTimeout> | null = null;
	let socket: SocketLike | null = null;
	// Two lifetimes, two counters: a query change invalidates the local fetch
	// AND the pass, but submit invalidates only the pass -- collapsing these
	// into one counter is what let submit discard a local fetch still wanted.
	let queryGeneration = 0;
	let passGeneration = 0;
	let disposed = false;

	function clearIdle(): void {
		if (idleTimer !== null) clearTimeout(idleTimer);
		idleTimer = null;
	}

	/** Closing the socket is the cancel -- there is no cancel endpoint (spec 1.3). */
	function stopPass(): void {
		if (pollTimer !== null) clearTimeout(pollTimer);
		pollTimer = null;
		socket?.close();
		socket = null;
	}

	function cancelPass(): void {
		passGeneration++;
		clearIdle();
		stopPass();
	}

	function cancelAll(): void {
		queryGeneration++;
		cancelPass();
	}

	/**
	 * Sorting and narrowing (spec 9.6) act on the answer the client is holding,
	 * so the answer has to be the whole answer. Core defaults `limit` to 25 and
	 * caps it at 100; asking for the cap makes the set complete for any query
	 * that does not match more arrows than exist on the machine plus one pass.
	 */
	function localPath(): string {
		return `/v0/search?q=${encodeURIComponent(query)}&limit=${SEARCH_LIMIT}`;
	}

	async function runLocal(myGeneration: number): Promise<void> {
		if (isBlank(query)) return;
		try {
			const dtos = await apiFetch<SearchResultDTO[]>(localPath());
			if (disposed || queryGeneration !== myGeneration) return;
			store.getState().setLocal(dtos.map(toSearchEntry));
		} catch {
			if (disposed || queryGeneration !== myGeneration) return;
			store.getState().setLocalError();
		}
	}

	/** Streamed results are unranked (spec 3); Lane A now sees the vault the pass just filled. */
	async function requery(myGeneration: number): Promise<void> {
		try {
			const dtos = await apiFetch<SearchResultDTO[]>(localPath());
			if (disposed || passGeneration !== myGeneration) return;
			store.getState().settle(dtos.map(toSearchEntry));
		} catch {
			if (disposed || passGeneration !== myGeneration) return;
			store.getState().settleFailed();
		}
	}

	async function startPass(myGeneration: number): Promise<void> {
		if (isBlank(query)) return;

		const started = await apiFetch<DiscoveryJobStartedDTO>('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q: query }),
		}).catch(() => null);

		if (!started || disposed || passGeneration !== myGeneration) return;

		store.getState().beginPass({ id: started.job_id, expires_at: started.expires_at });

		const path = `/v0/search/discover/${started.job_id}`;
		socket = backend().openSocket(path);
		socket.onmessage = (event) => {
			if (disposed || passGeneration !== myGeneration) return;
			try {
				store.getState().receive(toSearchEntry(JSON.parse(event.data) as SearchResultDTO));
			} catch {
				// A frame we cannot parse is not worth tearing the pass down for.
			}
		};

		const deadline = Date.now() + PASS_DEADLINE_MS;

		// Recursive, not setInterval: the next poll is scheduled only once this one
		// settles, so a round trip slower than POLL_INTERVAL_MS can never leave two
		// polls in flight to both observe `completed` and both consume the summary.
		function poll(): void {
			pollTimer = setTimeout(() => {
				if (disposed || passGeneration !== myGeneration) return;

				if (Date.now() >= deadline) {
					stopPass();
					store.getState().settleFailed();
					return;
				}

				void apiFetch<DiscoveryJobDTO>(path)
					.then((job) => {
						if (disposed || passGeneration !== myGeneration) return;
						if (job.status !== 'completed') {
							poll();
							return;
						}

						// This response IS the summary: one call, not two.
						stopPass();
						store.getState().endPass(toDiscoverySummary(job));
						void requery(myGeneration);
					})
					.catch(() => {
						if (disposed || passGeneration !== myGeneration) return;
						stopPass();
						store.getState().settleFailed();
					});
			}, POLL_INTERVAL_MS);
		}

		poll();
	}

	function arm(): void {
		clearIdle();
		if (isBlank(query)) return;
		const myGeneration = passGeneration;
		idleTimer = setTimeout(() => {
			if (disposed || passGeneration !== myGeneration) return;
			void startPass(myGeneration);
		}, IDLE_BEFORE_PASS_MS);
	}

	return {
		setQuery: (next, options) => {
			if (disposed || next === query) return;
			query = next;
			cancelAll();
			store.getState().setQuery(next);
			if (isBlank(next)) return;
			void runLocal(queryGeneration);
			if (options?.discover !== false) arm();
		},

		submit: () => {
			if (disposed || isBlank(query)) return;
			cancelPass();
			void startPass(passGeneration);
		},

		dispose: () => {
			cancelAll();
			disposed = true;
			store.getState().clear();
		},
	};
}
