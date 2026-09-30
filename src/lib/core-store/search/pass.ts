import { resolveRealPlatform } from '@/features/arrow-details/lib/use-real-platform';
import { apiFetch } from '@/lib/transport/api';
import { backend, type SocketCloseInfo, type SocketLike } from '@/lib/transport/backend';

import type { DiscoveryJobDTO, DiscoveryJobStartedDTO, SearchResultDTO } from '../dtos/v0/search';
import { toDiscoverySummary, toSearchEntry } from '../dtos/v0/search';
import { useSearchStore } from '../store/search';

/** Stillness before a provider pass. Measured from the committed query -- spec 2.2.1. */
export const IDLE_BEFORE_PASS_MS = 600;
/**
 * How long a reconnected stream may take to close. The clean close frame is the
 * completion signal; this only bounds a pass whose stream broke abnormally and
 * whose replay never reaches one.
 */
export const PASS_DEADLINE_MS = 25_000;
/** A dropped stream is replayed by core, so one reconnect recovers it. */
const MAX_RECONNECTS = 1;
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
	let safetyTimer: ReturnType<typeof setTimeout> | null = null;
	let platformKey: Promise<string> | null = null;
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
		if (safetyTimer !== null) clearTimeout(safetyTimer);
		safetyTimer = null;
		if (socket) {
			socket.onclose = null;
			socket.close();
		}
		socket = null;
	}

	/** Both lanes filter on the same platform, or the stream and the re-query cover different sets. */
	function platform(): Promise<string> {
		platformKey ??= resolveRealPlatform();
		return platformKey;
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
	async function localPath(): Promise<string> {
		const os = encodeURIComponent(await platform());
		return `/v0/search?q=${encodeURIComponent(query)}&limit=${SEARCH_LIMIT}&os=${os}`;
	}

	async function runLocal(myGeneration: number): Promise<void> {
		if (isBlank(query)) return;
		try {
			const dtos = await apiFetch<SearchResultDTO[]>(await localPath());
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
			const dtos = await apiFetch<SearchResultDTO[]>(await localPath());
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
		const os = encodeURIComponent(await platform());
		if (disposed || passGeneration !== myGeneration) return;

		const isCurrent = (): boolean => !disposed && passGeneration === myGeneration;

		/** The close frame is the end of the stream; the job summary is readable from then on. */
		async function complete(): Promise<void> {
			stopPass();
			try {
				const job = await apiFetch<DiscoveryJobDTO>(path);
				if (!isCurrent()) return;
				store.getState().endPass(toDiscoverySummary(job));
			} catch {
				if (!isCurrent()) return;
			}
			void requery(myGeneration);
		}

		function connect(reconnects: number): void {
			const current = backend().openSocket(`${path}?os=${os}`);
			socket = current;

			current.onmessage = (event) => {
				if (!isCurrent()) return;
				try {
					store.getState().receive(toSearchEntry(JSON.parse(event.data) as SearchResultDTO));
				} catch {
					// A frame we cannot parse is not worth tearing the pass down for.
				}
			};

			current.onclose = (info?: SocketCloseInfo) => {
				if (!isCurrent() || socket !== current) return;

				if (info?.code === 1000 && info.reason === 'completed') {
					void complete();
					return;
				}

				if (reconnects >= MAX_RECONNECTS) {
					stopPass();
					store.getState().settleFailed();
					return;
				}

				// Core replays everything the pass emitted, and `receive` dedups on the
				// bare namespace, so a reconnect is safe to repeat over what we hold.
				socket = null;
				safetyTimer ??= setTimeout(() => {
					if (!isCurrent()) return;
					stopPass();
					store.getState().settleFailed();
				}, PASS_DEADLINE_MS);
				connect(reconnects + 1);
			};
		}

		connect(0);
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
