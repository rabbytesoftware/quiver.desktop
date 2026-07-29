import type { ActiveRun, ArrowState, RuntimeUpdate, StepProgress } from '@/domain/arrow';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

export interface InstalledVersionDTO {
	ref: string;
	version: string;
	state: ArrowState;
	installed_at?: string;
}

export interface ArrowListResponseItemDTO {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	media?: {
		icon?: string | null;
		banner?: string | null;
	};
	versions: InstalledVersionDTO[];
}

export interface LastReturnDTO {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

/** Shape of GET /arrow/{ns} detail response. */
export interface ArrowDetailDTO {
	namespace: string;
	name: string;
	version: string;
	description: string;
	license: string;
	state: ArrowState;
	tags: string[];
	installed_ref: string;
	installed_at: string;
	installed_constraint?: string;
	user_installed: boolean;
	active_run?: ActiveRun | null;
	last_return?: LastReturnDTO | null;
}

/**
 * Catalog records only — runtime state is deliberately excluded (design §5.4).
 *
 * `media` is nested in stable-26.5.1. The previous flat `icon`/`banner` fields
 * never matched the wire, so every arrow silently carried null media.
 */
export function toArrowCatalogRecords(items: ArrowListResponseItemDTO[], connectionId: string): ArrowCatalogRecord[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			connectionId,
			namespace: `${arrow.namespace}@${v.ref}`,
			name: arrow.name,
			description: arrow.description,
			tags: arrow.tags,
			icon: arrow.media?.icon || null,
			banner: arrow.media?.banner || null,
			version: v.version,
		}))
	);
}

/**
 * Initial runtime state, sourced from the SAME seed GET as the catalog.
 *
 * Neither `/v0/arrow` nor `/v0/runtime` push anything on connect — both are
 * transition-only sockets (verified directly against stable-26.5.1). Without
 * this, every installed/running arrow would render as `'absent'` forever,
 * because the store's own neutral default has no other way to learn the
 * current state until *something* transitions. `versions[].state` is the
 * only place that current state exists at seed time — the list DTO carries
 * no `active_run`/`last_return` at all (those are detail-only fields), so
 * both are always null here; a live `/v0/runtime` frame fills them in once
 * something actually happens.
 */
export function toInitialRuntimeUpdates(items: ArrowListResponseItemDTO[]): RuntimeUpdate[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			namespace: `${arrow.namespace}@${v.ref}`,
			state: v.state,
			active_run: null,
			last_return: null,
		}))
	);
}
