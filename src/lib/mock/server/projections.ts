// World → wire. Everything the handlers return goes through here, so the DTO
// shapes live in one file and a change to quiver.core lands in one place.

import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '@/lib/core-store/dtos/v0/runtime';

import type { MockArrow, MockCollection, MockDiscoveryJob } from '../world/types';
import { versioned } from '../world/types';

/**
 * `GET /v0/arrow` groups by BASE namespace and nests refs under `versions`.
 *
 * Not a stylistic choice: `toArrowCatalogRecords` reads `arrow.namespace` and
 * `v.ref` and joins them itself, so a flat list keyed by versioned namespace
 * would come out the other side as `github.com/x/y@v1@v1`.
 */
export function toArrowListDTO(arrows: MockArrow[]): ArrowListResponseItemDTO[] {
	const byBase = new Map<string, MockArrow[]>();
	for (const arrow of arrows) {
		const group = byBase.get(arrow.namespace) ?? [];
		group.push(arrow);
		byBase.set(arrow.namespace, group);
	}

	return [...byBase.entries()].map(([namespace, group]) => ({
		namespace,
		name: group[0].name,
		description: group[0].description,
		tags: group[0].tags,
		// Nested, matching stable-26.5.1. The flat `icon`/`banner` this once used
		// never matched the wire, so every arrow silently carried null media.
		media: { icon: group[0].icon, banner: group[0].banner },
		versions: group.map((a) => ({
			ref: a.ref,
			version: a.version,
			state: a.state,
			installed_at: a.installed_at,
		})),
	}));
}

/**
 * A `/v0/arrow` WS frame. `event` is the transport verb; the catalog fields
 * ride along on an upsert and are absent on a tombstone.
 */
export function toArrowFrame(arrow: MockArrow, event: 'upserted' | 'removed'): unknown {
	if (event === 'removed') return { event, namespace: versioned(arrow) };
	return {
		event,
		namespace: versioned(arrow),
		name: arrow.name,
		description: arrow.description,
		tags: arrow.tags,
		media: { icon: arrow.icon, banner: arrow.banner },
		version: arrow.version,
	};
}

/** A `/v0/runtime` overlay frame. Patches state onto an entry that must exist. */
export function toRuntimeFrame(arrow: MockArrow): RuntimeUpdateDTO {
	return {
		namespace: versioned(arrow),
		state: arrow.state,
		active_run: arrow.active_run,
		last_return: arrow.last_return
			? { method: arrow.last_return.method, outcome: arrow.last_return.outcome }
			: null,
	};
}

/**
 * `GET /v0/arrow/{ns}`.
 *
 * A superset of core's `ArrowDetailDTO`: the manifest halves the inspection
 * screen needs — variables, targets, netbridge, requirement — come back on the
 * same read rather than as a second round trip, which is what core's own
 * `arrow_detail_dto.go` already carries for variables and targets.
 *
 * `license` is populated here. Core DECLARES the field on its detail DTO and
 * never assigns it, so against a real daemon it is always empty; that is a core
 * bug on the ask-list, not something the mock should reproduce — a mock that
 * imitated it would hide the bug rather than surface it.
 */
export function toArrowDetailDTO(arrow: MockArrow): unknown {
	return {
		namespace: arrow.namespace,
		name: arrow.name,
		version: arrow.version,
		description: arrow.description,
		license: arrow.license,
		state: arrow.state,
		tags: arrow.tags,
		installed_ref: arrow.ref,
		installed_at: arrow.installed_at,
		user_installed: arrow.user_installed,
		active_run: arrow.active_run,
		last_return: arrow.last_return,
		media: { icon: arrow.icon, banner: arrow.banner },
		maintainers: arrow.maintainers,
		url: arrow.url,
		requirement: arrow.requirement,
		netbridge: arrow.netbridge,
		variables: arrow.variables,
		targets: arrow.targets,
	};
}

/** `GET /v0/search` — no state and no ports, matching core's `SearchResultDTO`. */
export function toSearchResultDTO(arrow: MockArrow): unknown {
	return {
		namespace: versioned(arrow),
		name: arrow.name,
		description: arrow.description,
		tags: arrow.tags,
		media: { icon: arrow.icon, banner: arrow.banner },
		maintainers: arrow.maintainers,
		version: arrow.version,
	};
}

export function toCollectionListDTO(collection: MockCollection): unknown {
	return {
		namespace: collection.namespace,
		name: collection.name,
		description: collection.description,
		maintainers: collection.maintainers,
		followed: collection.followed,
		arrow_count: collection.arrows.length,
	};
}

export function toCollectionDetailDTO(collection: MockCollection): unknown {
	return {
		...(toCollectionListDTO(collection) as object),
		arrows: collection.arrows,
	};
}

export function toDiscoveryJobDTO(job: MockDiscoveryJob): unknown {
	return {
		id: job.id,
		status: job.status,
		query: job.query,
		providers: job.providers,
		results: job.results,
	};
}
