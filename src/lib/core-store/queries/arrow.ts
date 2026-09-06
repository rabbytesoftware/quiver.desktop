import { useQuery } from '@tanstack/react-query';

import type { ArrowDetail } from '@/domain/arrow';
import { splitNamespace } from '@/lib/namespace';
import { apiFetch, isNotFoundError } from '@/lib/transport/api';

import type {
	ArrowDependenciesDTO,
	ArrowDependencyDTO,
	ArrowDependentsDTO,
	ArrowDetailDTO,
	ArrowManifestDTO,
	ArrowReadmeDTO,
} from '../dtos/v0/arrow';
import { toArrowDetail } from '../dtos/v0/arrow';

export const arrowDetailQueryKeyPrefix = ['arrow'] as const;

export function arrowDetailQueryKey(namespace: string) {
	return [...arrowDetailQueryKeyPrefix, namespace] as const;
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
 * Combines the five real endpoints quiver.core exposes for a single arrow --
 * `GET /v0/arrow/:ns` (state/active_run/last_return), `GET /v0/arrow/:ns/manifest`
 * (media, maintainers, credits, url, requirements, netbridge, variables,
 * methods), `GET /v0/arrow/:ns/readme` (ARROW.md prose), and
 * `GET /v0/arrow/:ns/dependencies` + `GET /v0/arrow/:ns/dependents`
 * (the dependency graph, quiver.core #220). There is no single endpoint that
 * returns all five; don't add one to the mock as a shortcut, since that
 * would stop the mock from catching a client that assumes there is.
 *
 * Both readme and manifest take the bare namespace -- core rejects a
 * `namespace@ref` path on either (`ErrInvalidNamespace`, 400), confirmed
 * live against the real daemon. The two dependency calls take the full
 * `namespace@ref`, since the resolved plan is version-specific.
 *
 * `versions` (the version-switcher's list) intentionally comes back empty --
 * it belongs to the catalog, not any of these calls, and the calling screen
 * should derive it from the already-loaded `useArrowStore` instead of a
 * sixth fetch.
 */
export function useArrowDetail(namespace: string) {
	return useQuery<ArrowDetail>({
		queryKey: arrowDetailQueryKey(namespace),
		queryFn: async () => {
			const bareNamespace = splitNamespace(namespace).head;
			const [detail, manifest, readme, dependencies, dependents] = await Promise.all([
				apiFetch<ArrowDetailDTO>(`/v0/arrow/${encodeURIComponent(namespace)}`),
				apiFetch<ArrowManifestDTO>(`/v0/arrow/${encodeURIComponent(bareNamespace)}/manifest`),
				fetchReadme(bareNamespace),
				fetchDependencies(namespace),
				fetchDependents(namespace),
			]);
			return toArrowDetail(detail, manifest, [], readme, dependencies, dependents);
		},
		enabled: namespace.length > 0,
	});
}
