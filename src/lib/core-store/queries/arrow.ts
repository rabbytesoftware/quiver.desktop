import { useQuery } from '@tanstack/react-query';

import type { ArrowChannel, ArrowDependency, ArrowDetail } from '@/domain/arrow';
import { splitNamespace } from '@/lib/namespace';
import { apiFetch, isNotFoundError } from '@/lib/transport/api';

import type {
	ArrowDependenciesDTO,
	ArrowDependencyDTO,
	ArrowDependentsDTO,
	ArrowDetailDTO,
	ArrowManifestDTO,
	ArrowReadmeDTO,
	ChannelListDTO,
} from '../dtos/v0/arrow';
import { toArrowChannels, toArrowDependencies, toArrowDetail } from '../dtos/v0/arrow';

export const arrowDetailQueryKeyPrefix = ['arrow'] as const;

export function arrowDetailQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace] as const;
}

export function arrowChannelsQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace, 'channels'] as const;
}

export function arrowReadmeQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace, 'readme'] as const;
}

export function arrowDependenciesQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace, 'dependencies'] as const;
}

export function arrowDependentsQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace, 'dependents'] as const;
}

/**
 * `GET /v0/arrow/:ns/readme` (quiver.core #219) 404s when the arrow has no
 * readme -- a plain `arrow.yaml` delivery, or an ARROW.md with nothing
 * outside its fenced block -- which is an expected outcome here, not a
 * failure of the whole detail fetch.
 */
async function fetchReadme(bareNamespace: string): Promise<string | null> {
	try {
		const dto = await apiFetch<ArrowReadmeDTO>(`/v0/arrow/${encodeURIComponent(bareNamespace)}/readme`);
		return dto.readme;
	} catch (err) {
		if (isNotFoundError(err)) return null;
		throw err;
	}
}

/**
 * `GET /v0/arrow/:ns/channels` -- like `/manifest` and `/readme`, this is a
 * repo-level property, not a per-ref one, so it takes the bare namespace.
 * 404 means the arrow itself doesn't resolve (or hasn't published any
 * channels), which is a fine "nothing to show" outcome here, not a failure
 * of the whole detail fetch -- same treatment as readme/dependencies/dependents.
 */
async function fetchChannels(bareNamespace: string): Promise<ChannelListDTO> {
	try {
		return await apiFetch<ChannelListDTO>(`/v0/arrow/${encodeURIComponent(bareNamespace)}/channels`);
	} catch (err) {
		if (isNotFoundError(err)) return { channels: [] };
		throw err;
	}
}

/**
 * `GET /v0/arrow/:ns/dependencies` (quiver.core #220) -- unlike readme/manifest,
 * this takes the full `namespace@ref`: the resolved plan is version-specific
 * (which ref's manifest declared which tools/services). 404 means the arrow
 * itself doesn't resolve, not "no dependencies" (that's just an empty array
 * on a 200) -- either way, an empty list is a fine fallback for this page.
 */
async function fetchDependencies(namespace: string): Promise<ArrowDependencyDTO[]> {
	try {
		const dto = await apiFetch<ArrowDependenciesDTO>(`/v0/arrow/${encodeURIComponent(namespace)}/dependencies`);
		return dto.dependencies;
	} catch (err) {
		if (isNotFoundError(err)) return [];
		throw err;
	}
}

/** `GET /v0/arrow/:ns/dependents` (quiver.core #220) -- core normalizes `:ns` to bare internally, but the full namespace@ref is passed for consistency with `/dependencies`. */
async function fetchDependents(namespace: string): Promise<string[]> {
	try {
		const dto = await apiFetch<ArrowDependentsDTO>(`/v0/arrow/${encodeURIComponent(namespace)}/dependents`);
		return dto.dependents;
	} catch (err) {
		if (isNotFoundError(err)) return [];
		throw err;
	}
}

/**
 * The FAST/primary half of the six real endpoints quiver.core exposes for a
 * single arrow -- `GET /v0/arrow/:ns` (state/active_run/last_return/channel)
 * and `GET /v0/arrow/:ns/manifest` (media, maintainers, credits, url,
 * requirements, netbridge, variables, methods). Both are simple, fast reads
 * (confirmed live: tens of milliseconds), unlike the other four (readme,
 * channels, the dependency graph -- `dependencies` especially, a real git
 * resolution that can run 1.5-2+ seconds). This is deliberately the ONLY
 * query `arrow-details-screen.tsx`'s top-level `isLoading`/`isError` gate
 * reflects -- identity, status, and the action row shouldn't sit behind a
 * spinner waiting on a git resolution they don't need.
 *
 * Returns a full `ArrowDetail` immediately once these two resolve, with the
 * four slow fields defaulted to their empty/absent shape (`channels: []`,
 * `readme: null`, `dependencies: []`, `dependents: []`) -- see
 * `useArrowChannels`/`useArrowReadme`/`useArrowDependencies`/`useArrowDependents`
 * below, and `arrow-details-screen.tsx`'s own merge, for how each of those
 * overlays its real value once it resolves. There is no single endpoint that
 * returns all six; don't add one to the mock as a shortcut, since that would
 * stop the mock from catching a client that assumes there is -- splitting
 * this into five independently-loading queries doesn't change that, it just
 * stops the frontend from gating everything on the slowest one.
 *
 * Manifest takes the bare namespace -- core rejects a `namespace@ref` path on
 * it (`ErrInvalidNamespace`, 400), confirmed live against the real daemon.
 */
export function useArrowDetail(namespace: string) {
	return useQuery<ArrowDetail>({
		queryKey: arrowDetailQueryKey(namespace),
		queryFn: async () => {
			const bareNamespace = splitNamespace(namespace).head;
			const [detail, manifest] = await Promise.all([
				apiFetch<ArrowDetailDTO>(`/v0/arrow/${encodeURIComponent(namespace)}`),
				apiFetch<ArrowManifestDTO>(`/v0/arrow/${encodeURIComponent(bareNamespace)}/manifest`),
			]);
			return toArrowDetail(detail, manifest, [], null, [], []);
		},
		enabled: namespace.length > 0,
	});
}

/**
 * SLOW/secondary: `GET /v0/arrow/:ns/channels`, own query, own loading state --
 * feeds the Hero's Channel/Version selects, which show their own
 * loading/disabled treatment while this is in flight rather than blocking the
 * whole page (see `channelsLoading` threaded through `Hero`/`ChannelVersionSelects`).
 * Bare namespace, like `/manifest` and `/readme` -- channels are a repo-level
 * property, not a per-ref one.
 */
export function useArrowChannels(namespace: string) {
	return useQuery<ArrowChannel[]>({
		queryKey: arrowChannelsQueryKey(namespace),
		queryFn: async () => toArrowChannels(await fetchChannels(splitNamespace(namespace).head)),
		enabled: namespace.length > 0,
	});
}

/**
 * SLOW/secondary: `GET /v0/arrow/:ns/readme`, own query, own loading state --
 * the Overview tab shows its own loading placeholder while this (and the two
 * dependency-graph queries below) are in flight, independent of the rest of
 * the page. `null` once resolved with nothing to show (see `fetchReadme`);
 * `undefined` (React Query's own "not yet resolved" value) is what the
 * screen's own loading placeholder keys off, not this field.
 */
export function useArrowReadme(namespace: string) {
	return useQuery<string | null>({
		queryKey: arrowReadmeQueryKey(namespace),
		queryFn: () => fetchReadme(splitNamespace(namespace).head),
		enabled: namespace.length > 0,
	});
}

/**
 * SLOW/secondary: `GET /v0/arrow/:ns/dependencies` (quiver.core #220) -- the
 * one of the four call out by name as genuinely slow: a real git resolution
 * on the backend, 1.5-2+ seconds observed live (now being cached
 * backend-side too, in a parallel fix). Own query, own loading state, feeds
 * the Overview tab's `MetaPanel` fallback (and its "depends on" rows) --
 * never gates the fast content above it.
 */
export function useArrowDependencies(namespace: string) {
	return useQuery<ArrowDependency[]>({
		queryKey: arrowDependenciesQueryKey(namespace),
		queryFn: async () => toArrowDependencies(await fetchDependencies(namespace)),
		enabled: namespace.length > 0,
	});
}

/** SLOW/secondary: `GET /v0/arrow/:ns/dependents` (quiver.core #220) -- own query, own loading state, feeds the Overview tab's `MetaPanel` fallback's "required by" rows. */
export function useArrowDependents(namespace: string) {
	return useQuery<string[]>({
		queryKey: arrowDependentsQueryKey(namespace),
		queryFn: () => fetchDependents(namespace),
		enabled: namespace.length > 0,
	});
}
