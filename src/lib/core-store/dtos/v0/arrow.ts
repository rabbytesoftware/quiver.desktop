import type { ActiveRun, ArrowState, StepProgress } from '@/domain/arrow';
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
