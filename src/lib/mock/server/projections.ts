import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '@/lib/core-store/dtos/v0/runtime';

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
