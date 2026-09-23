import { useEffect, useMemo, useRef } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowDetail, ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import {
	arrowDetailQueryKeyPrefix,
	useArrowChannels,
	useArrowDependencies,
	useArrowDependents,
	useArrowDetail,
	useArrowReadme,
} from '@/lib/core-store/queries/arrow';

export interface AssembledArrowDetail {
	/** The merged `ArrowDetail` -- fast fields plus whichever slow fields have resolved so far, live-store state overlaid on top. `undefined` while the fast query hasn't resolved yet. */
	detail: ArrowDetail | undefined;
	/** Reflects ONLY the fast `detail` + `manifest` query -- identity, status, and the action row never wait on the four slow ones. */
	isLoading: boolean;
	isError: boolean;
	/** True while any of readme/dependencies/dependents is still in flight -- the Overview tab's own loading placeholder, independent of the rest of the page. */
	overviewLoading: boolean;
	/** True while `GET /v0/arrow/:ns/channels` is still in flight -- the Hero's Channel/Version selects' own loading state. */
	channelsLoading: boolean;
	/** The reactive store's full arrow map, for `buildDependencyRows` to fill in name/icon/live state on the dependency graph's bare namespaces. */
	allEntries: Map<string, ArrowEntry>;
}

/**
 * Composes the FAST/primary `useArrowDetail` query with the four SLOW/secondary
 * ones (channels, readme, dependencies, dependents) plus the reactive store's
 * live overlay, into the one `ArrowDetail` shape `arrow-details-screen.tsx`'s
 * component tree already consumes -- pulled out of that screen itself
 * (react-doctor's `no-giant-component`, the same reason `Hero`'s own
 * channel-selection logic lives in `use-channel-selection.ts`).
 *
 * Each slow field defaults to the fast query's own placeholder (`[]`/`null`)
 * until its own query resolves; a section that needs to tell "not yet loaded"
 * apart from "loaded, genuinely nothing" reads `overviewLoading`/`channelsLoading`
 * for that, never the merged `detail` fields themselves.
 */
export function useAssembledArrowDetail(namespace: string): AssembledArrowDetail {
	const queryClient = useQueryClient();
	const { data, isLoading, isError } = useArrowDetail(namespace);
	const channelsQuery = useArrowChannels(namespace);
	const readmeQuery = useArrowReadme(namespace);
	const dependenciesQuery = useArrowDependencies(namespace);
	const dependentsQuery = useArrowDependents(namespace);
	const overviewLoading = readmeQuery.isLoading || dependenciesQuery.isLoading || dependentsQuery.isLoading;

	// The reactive store only ever holds arrows the user has added
	// (listeners/index.ts seeds it from `user_installed=true` only) -- for a
	// Discovered arrow, `liveEntry` stays undefined and `data`'s own
	// one-time-fetched state/active_run/last_return are used as-is, which is
	// correct: there is nothing live to overlay.
	const liveEntry = useArrowStore((state) => state.arrows.get(namespace));
	const allEntries = useArrowStore((state) => state.arrows);

	const previousRunState = useRef<{ namespace: string; active: boolean }>({ namespace, active: false });
	useEffect(() => {
		const isActive = liveEntry?.active_run !== null && liveEntry?.active_run !== undefined;
		const previous = previousRunState.current;
		if (previous.namespace === namespace && previous.active && !isActive) {
			void queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
		}
		previousRunState.current = { namespace, active: isActive };
	}, [namespace, liveEntry?.active_run, queryClient]);

	const detail = useMemo(() => {
		if (!data) return data;
		// The live overlay's `last_return` (from the WS runtime-update frame)
		// deliberately carries no `steps` -- core omits them there to avoid
		// pushing full step history on every transition (see LastReturnDetail's
		// own comment in src/domain/arrow.ts). Reuse the richer, one-time-fetched
		// `steps`/`variables` only when they're actually describing the same
		// run; a live push reporting a genuinely new outcome falls back to an
		// empty step list rather than showing stale, mismatched detail.
		const liveLastReturn = liveEntry?.last_return;
		const sameRun =
			liveLastReturn &&
			data.last_return?.method === liveLastReturn.method &&
			data.last_return?.outcome === liveLastReturn.outcome;
		return {
			...data,
			state: liveEntry?.state ?? data.state,
			active_run: liveEntry?.active_run ?? data.active_run,
			last_return: liveLastReturn
				? {
						...liveLastReturn,
						variables: sameRun ? data.last_return!.variables : {},
						steps: sameRun ? data.last_return!.steps : [],
					}
				: data.last_return,
			channels: channelsQuery.data ?? data.channels,
			readme: readmeQuery.data ?? data.readme,
			dependencies: dependenciesQuery.data ?? data.dependencies,
			dependents: dependentsQuery.data ?? data.dependents,
		};
	}, [data, liveEntry, channelsQuery.data, readmeQuery.data, dependenciesQuery.data, dependentsQuery.data]);

	return {
		detail,
		isLoading,
		isError,
		overviewLoading,
		channelsLoading: channelsQuery.isLoading,
		allEntries,
	};
}
