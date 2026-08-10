import type { ArrowState, RuntimeUpdate } from '@/domain/arrow';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

interface InstalledVersionDTO {
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
