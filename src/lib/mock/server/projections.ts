import type { SearchProvenance } from '@/domain/search';
import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '@/lib/core-store/dtos/v0/runtime';
import type { DiscoveryJobDTO, DiscoveryJobStartedDTO, SearchResultDTO } from '@/lib/core-store/dtos/v0/search';

import type { MockArrow, MockCollection, MockDiscoveryJob } from '../world/types';
import { versioned } from '../world/types';

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
		media: { icon: group[0].icon, banner: group[0].banner },
		versions: group.map((a) => ({
			ref: a.ref,
			version: a.version,
			state: a.state,
			installed_at: a.installed_at,
		})),
	}));
}

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

/**
 * One search result, in the single shape both lanes share.
 *
 * `refs` is passed in rather than read off the arrow because a namespace can
 * have several installed refs, and core reports them together on one row --
 * `arrow` is only the row's representative.
 */
export function toSearchResultDTO(
	arrow: MockArrow,
	facts: { refs: string[]; installed: boolean; known: boolean; provenance?: SearchProvenance }
): SearchResultDTO {
	return {
		namespace: arrow.namespace,
		name: arrow.name,
		description: arrow.description,
		tags: arrow.tags,
		media: { icon: arrow.icon, banner: arrow.banner },
		versions: facts.refs,
		compatible_os: [...new Set(arrow.targets.map((t) => t.platform))].sort(),
		...(facts.provenance ? { provenance: facts.provenance } : {}),
		installed: facts.installed,
		known: facts.known,
		stars: arrow.stars ?? 0,
		...(arrow.source ? { source: arrow.source } : {}),
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

/** The ticket, returned before any host has been asked. Carries no counts. */
export function toDiscoveryStartedDTO(job: MockDiscoveryJob): DiscoveryJobStartedDTO {
	return { job_id: job.id, query: job.query, expires_at: job.expires_at };
}

/** The summary. Results are not here: they left over the socket. */
export function toDiscoveryJobDTO(job: MockDiscoveryJob): DiscoveryJobDTO {
	return {
		job_id: job.id,
		status: job.status,
		query: job.query,
		found: job.found,
		verified: job.verified,
		skipped: job.skipped,
		providers: job.providers,
	};
}
