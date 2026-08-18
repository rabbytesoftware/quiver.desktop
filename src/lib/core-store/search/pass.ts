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

export interface SearchController {
	/** Called with a committed (URL) query -- already debounced by the field. */
	setQuery: (query: string) => void;
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
	let generation = 0;
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

	function cancelAll(): void {
		generation++;
		clearIdle();
		stopPass();
	}

	async function runLocal(myGeneration: number): Promise<void> {
		if (isBlank(query)) return;
		try {
			const dtos = await apiFetch<SearchResultDTO[]>(`/v0/search?q=${encodeURIComponent(query)}`);
			if (disposed || generation !== myGeneration) return;
			store.getState().setLocal(dtos.map(toSearchEntry));
		} catch {
			if (disposed || generation !== myGeneration) return;
			store.getState().setLocalError();
		}
	}

	/** Streamed results are unranked (spec 3); Lane A now sees the vault the pass just filled. */
	async function requery(myGeneration: number): Promise<void> {
		try {
			const dtos = await apiFetch<SearchResultDTO[]>(`/v0/search?q=${encodeURIComponent(query)}`);
			if (disposed || generation !== myGeneration) return;
			store.getState().settle(dtos.map(toSearchEntry));
		} catch {
			if (disposed || generation !== myGeneration) return;
			store.getState().settleFailed();
		}
	}

	async function startPass(myGeneration: number): Promise<void> {
		if (isBlank(query)) return;

		const started = await apiFetch<DiscoveryJobStartedDTO>('/v0/search/discover', {
			method: 'POST',
			body: JSON.stringify({ q: query }),
		}).catch(() => null);

		if (!started || disposed || generation !== myGeneration) return;

		store.getState().beginPass({ id: started.job_id, expires_at: started.expires_at });

		const path = `/v0/search/discover/${started.job_id}`;
		socket = backend().openSocket(path);
		socket.onmessage = (event) => {
			if (disposed || generation !== myGeneration) return;
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
				if (disposed || generation !== myGeneration) return;

				if (Date.now() >= deadline) {
					stopPass();
					store.getState().settleFailed();
					return;
				}

				void apiFetch<DiscoveryJobDTO>(path)
					.then((job) => {
						if (disposed || generation !== myGeneration) return;
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
						if (disposed || generation !== myGeneration) return;
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
		const myGeneration = generation;
		idleTimer = setTimeout(() => {
			if (disposed || generation !== myGeneration) return;
			void startPass(myGeneration);
		}, IDLE_BEFORE_PASS_MS);
	}

	return {
		setQuery: (next) => {
			if (disposed || next === query) return;
			query = next;
			cancelAll();
			store.getState().setQuery(next);
			if (isBlank(next)) return;
			void runLocal(generation);
			arm();
		},

		submit: () => {
			if (disposed || isBlank(query)) return;
			cancelAll();
			void startPass(generation);
		},

		dispose: () => {
			cancelAll();
			disposed = true;
		},
	};
}
