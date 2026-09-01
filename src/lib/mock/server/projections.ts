import type { SearchProvenance } from '@/domain/search';
import type {
	ArrowDependenciesDTO,
	ArrowDependentsDTO,
	ArrowDetailDTO,
	ArrowListResponseItemDTO,
	ArrowManifestDTO,
	ArrowReadmeDTO,
} from '@/lib/core-store/dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '@/lib/core-store/dtos/v0/runtime';
import type { DiscoveryJobDTO, DiscoveryJobStartedDTO, SearchResultDTO } from '@/lib/core-store/dtos/v0/search';
import { splitNamespace } from '@/lib/namespace';

import { INSTALL_STEPS, START_STEPS, STOP_STEPS, UNINSTALL_STEPS, UPDATE_STEPS } from '../world/scenarios/kit';
import type { MockArrow, MockCollection, MockDiscoveryJob, MockMethod, MockTarget } from '../world/types';
import { versioned } from '../world/types';

// Every arrow's lifecycle preview uses this exact same step sequence per
// phase -- handlers/runtime.ts simulates every arrow's install/execute/stop/
// update/uninstall with this exact generic sequence regardless of its
// manifest, so what StepPreviewModal previews and what actually runs always
// agree. `StepDTO` is `ArrowStepDefinition` verbatim, so these need no
// reshaping to become the wire lifecycle -- they already are it.
const LIFECYCLE_STEPS = {
	install: INSTALL_STEPS,
	update: UPDATE_STEPS,
	execute: START_STEPS,
	stop: STOP_STEPS,
	uninstall: UNINSTALL_STEPS,
} as const;

function toMethodDTO(method: MockMethod): ArrowManifestDTO['targets'][string]['methods'][string] {
	return {
		name: method.name,
		description: method.description,
		available_in: method.available_in,
		steps: method.steps,
	};
}

function toTargetManifestDTO(
	target: MockTarget,
	requirement: MockArrow['requirement']
): ArrowManifestDTO['targets'][string] {
	return {
		requirements: requirement,
		lifecycle: {
			install: LIFECYCLE_STEPS.install,
			update: LIFECYCLE_STEPS.update,
			execute: LIFECYCLE_STEPS.execute,
			stop: LIFECYCLE_STEPS.stop,
			uninstall: LIFECYCLE_STEPS.uninstall,
		},
		methods: Object.fromEntries(Object.entries(target.methods).map(([name, m]) => [name, toMethodDTO(m)])),
	};
}

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

/**
 * `GET /v0/arrow/:ns` -- matches quiver.core's real `ArrowDetailDTO`
 * (internal/api/v0/dto/arrow_detail.go) exactly. No media, maintainers,
 * credits, url, requirements, netbridge, variables, or methods here -- those
 * only ever come from `toArrowManifestDTO` below, over the separate
 * `/manifest` endpoint. Keeping the two split, even in the mock, is what lets
 * a client that wrongly assumes one combined endpoint get caught here instead
 * of failing against real quiver.core.
 */
export function toArrowDetailDTO(arrow: MockArrow): ArrowDetailDTO {
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
	};
}

/** `GET /v0/arrow/:ns/manifest` -- matches quiver.core's real `ArrowManifestDTO` (internal/api/v0/dto/arrow_manifest.go). */
export function toArrowManifestDTO(arrow: MockArrow): ArrowManifestDTO {
	return {
		namespace: arrow.namespace,
		name: arrow.name,
		description: arrow.description,
		tags: arrow.tags,
		variables: arrow.variables,
		targets: Object.fromEntries(
			arrow.targets.map((target) => [target.platform, toTargetManifestDTO(target, arrow.requirement)])
		),
		manifest: {
			url: arrow.url,
			maintainers: arrow.maintainers.map((name) => ({ name })),
			credits: (arrow.credits ?? []).map((name) => ({ name })),
			media: { icon: arrow.icon ?? undefined, banner: arrow.banner ?? undefined },
			netbridge: arrow.netbridge,
		},
	};
}

/** `GET /v0/arrow/:ns/readme` -- matches quiver.core's real `ArrowReadmeDTO` (internal/api/v0/dto/arrow_readme.go, quiver.core #219). `:ns` here is always the bare namespace. */
export function toArrowReadmeDTO(bareNamespace: string, readme: string): ArrowReadmeDTO {
	return { namespace: bareNamespace, readme };
}

/** `GET /v0/arrow/:ns/dependencies` -- matches quiver.core's real `ArrowDependenciesDTO` (internal/api/v0/dto/arrow_dependencies.go, quiver.core #220). */
export function toArrowDependenciesDTO(arrow: MockArrow): ArrowDependenciesDTO {
	return {
		namespace: versioned(arrow),
		dependencies: (arrow.dependencies ?? []).map((dep) => ({ namespace: dep.namespace, type: dep.type })),
	};
}

/**
 * `GET /v0/arrow/:ns/dependents` -- matches quiver.core's real `ArrowDependentsDTO`
 * (internal/api/v0/dto/arrow_dependents.go, quiver.core #220). The mock has no
 * separate edge store, so this scans every other arrow's own declared
 * `dependencies` for one pointing back at this namespace -- that scan IS the
 * graph here, same as core's edge table is there.
 */
export function toArrowDependentsDTO(arrow: MockArrow, allArrows: MockArrow[]): ArrowDependentsDTO {
	const bare = arrow.namespace;
	const dependents: string[] = [];
	for (const candidate of allArrows) {
		const dependsOnThis = (candidate.dependencies ?? []).some((dep) => splitNamespace(dep.namespace).head === bare);
		if (dependsOnThis) dependents.push(versioned(candidate));
	}
	return { namespace: versioned(arrow), dependents };
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

/** Matches quiver.core's real CollectionListItemDTO -- no maintainers, no media at list level. */
export function toCollectionListDTO(collection: MockCollection): unknown {
	return {
		namespace: collection.namespace,
		name: collection.name,
		description: collection.description,
		tags: collection.tags,
		arrow_count: collection.arrows.length,
		followed: collection.followed,
	};
}

/** Matches quiver.core's real CollectionDetailDTO: url/media are only present on the wire when set. */
export function toCollectionDetailDTO(collection: MockCollection): unknown {
	return {
		namespace: collection.namespace,
		name: collection.name,
		description: collection.description,
		...(collection.url ? { url: collection.url } : {}),
		maintainers: collection.maintainers,
		tags: collection.tags,
		...(collection.media ? { media: collection.media } : {}),
		arrows: collection.arrows,
		followed: collection.followed,
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
