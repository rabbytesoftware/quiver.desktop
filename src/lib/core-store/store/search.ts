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
	/**
	 * The query the field reopened rather than asked for. Reopening restores a
	 * screen; it is not a new request, so Lane B stays put and Lane A rebuilds
	 * from the vault the last pass already filled. Carries the value, not a
	 * flag, so a query typed between the request and the mount is still treated
	 * as the new ask it is.
	 */
	restoreQuery: string | null;

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
	requestRestore: (query: string) => void;
	clearRestore: () => void;
	clear: () => void;
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

function phaseAfterLocal(phase: SearchPhase): SearchPhase {
	return phase === 'discovering' || phase === 'settling' || phase === 'settled' ? phase : 'local';
}

export const useSearchStore = create<SearchStore>((set, get) => ({
	query: '',
	submitQuery: null,
	restoreQuery: null,
	...EMPTY,

	setQuery: (query) => set({ query, ...EMPTY }),

	// Lane A resolves the platform before it asks, so a slow answer can land
	// after the pass has begun -- it must fill the band, not rewind the phase.
	setLocal: (local) => set({ local, phase: phaseAfterLocal(get().phase), localError: false }),

	setLocalError: () => set({ localError: true, phase: phaseAfterLocal(get().phase) }),

	beginPass: (job) => set({ job, phase: 'discovering', passFailed: false }),

	// Dedup on the bare namespace within the streamed band only. A namespace
	// `local` also holds is kept: whether the local copy is on screen is the
	// screen's call (it filters that lane to the query), and dropping the
	// streamed copy here would lose the arrow when it is not.
	receive: (entry) => {
		const { phase, streamed } = get();
		if (phase !== 'discovering') return;
		if (streamed.some((e) => e.namespace === entry.namespace)) return;
		set({ streamed: [...streamed, entry] });
	},

	endPass: (summary) => set({ summary, phase: 'settling' }),

	// Local is a replacement; `streamed` is left whole (arrival order). Some
	// probed results are deliberately never indexed, so the re-query alone would
	// erase them, and one it does return may be filtered out of the local band by
	// the screen -- which then still needs the streamed copy.
	settle: (local) => set({ local, phase: 'settled' }),

	// Holds the phase and keeps both bands: clearing them would delete
	// results the user can see, to recover from a failed re-query.
	settleFailed: () => set({ phase: 'settling', passFailed: true }),

	requestSubmit: (query) => set({ submitQuery: query }),

	clearSubmit: () => set({ submitQuery: null }),

	requestRestore: (restoreQuery) => set({ restoreQuery }),

	clearRestore: () => set({ restoreQuery: null }),

	// The screen went away, so its results go with it: a query and the results
	// attached to it belong to one mount, not to the app. Deliberately not
	// `reset` -- `submitQuery` is a message from the always-mounted field, and
	// dropping it here would swallow an Enter that navigated to a fresh screen.
	clear: () => set({ query: '', ...EMPTY }),

	reset: () => set({ query: '', submitQuery: null, restoreQuery: null, ...EMPTY }),
}));
