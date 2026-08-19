import { create } from 'zustand';

import type { DiscoverySummary, SearchEntry } from '@/domain/search';

export type SearchPhase = 'idle' | 'local' | 'discovering' | 'settling' | 'settled';

export interface SearchJob {
	id: string;
	expires_at: string;
}

interface SearchStore {
	query: string;
	phase: SearchPhase;
	/** Lane A, ranked. */
	local: SearchEntry[];
	/** Lane B, arrival order -- these have no score to rank by. */
	streamed: SearchEntry[];
	job: SearchJob | null;
	/** Held in memory: the job 404s 30s after the pass ends. */
	summary: DiscoverySummary | null;
	localError: boolean;
	passFailed: boolean;
	/**
	 * The query Enter was pressed on, or null. `SearchBar` and the mounted
	 * pass controller are siblings under the route tree, not parent/child, so
	 * this is how Enter reaches the controller (spec 2.2). It carries the
	 * exact committed value rather than relying on the URL, which lags behind
	 * an in-flight navigation.
	 */
	submitQuery: string | null;

	setQuery: (query: string) => void;
	setLocal: (entries: SearchEntry[]) => void;
	setLocalError: () => void;
	beginPass: (job: SearchJob) => void;
	receive: (entry: SearchEntry) => void;
	endPass: (summary: DiscoverySummary) => void;
	settle: (entries: SearchEntry[]) => void;
	settleFailed: () => void;
	requestSubmit: (query: string) => void;
	clearSubmit: () => void;
	reset: () => void;
}

const EMPTY = {
	phase: 'idle' as SearchPhase,
	local: [] as SearchEntry[],
	streamed: [] as SearchEntry[],
	job: null,
	summary: null,
	localError: false,
	passFailed: false,
};

export const useSearchStore = create<SearchStore>((set, get) => ({
	query: '',
	submitQuery: null,
	...EMPTY,

	setQuery: (query) => set({ query, ...EMPTY }),

	setLocal: (local) => set({ local, phase: 'local', localError: false }),

	setLocalError: () => set({ localError: true, phase: 'local' }),

	beginPass: (job) => set({ job, phase: 'discovering', passFailed: false }),

	// Dedup on the bare namespace against both bands; a namespace already in
	// `local` is dropped, never moved -- it's on screen already, ranked.
	receive: (entry) => {
		const { phase, local, streamed } = get();
		if (phase !== 'discovering') return;
		if (local.some((e) => e.namespace === entry.namespace)) return;
		if (streamed.some((e) => e.namespace === entry.namespace)) return;
		set({ streamed: [...streamed, entry] });
	},

	endPass: (summary) => set({ summary, phase: 'settling' }),

	// A replacement, not a patch: merging here would re-introduce the ordering
	// ambiguity the two bands exist to avoid.
	settle: (local) => set({ local, streamed: [], phase: 'settled' }),

	// Holds the phase and keeps both bands: clearing them would delete
	// results the user can see, to recover from a failed re-query.
	settleFailed: () => set({ phase: 'settling', passFailed: true }),

	requestSubmit: (query) => set({ submitQuery: query }),

	clearSubmit: () => set({ submitQuery: null }),

	reset: () => set({ query: '', submitQuery: null, ...EMPTY }),
}));
