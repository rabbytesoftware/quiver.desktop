import type { ArrowListItem, ArrowState, ActiveRun, LastOutcome, StepProgress } from '@/domain/arrow';

export interface InstalledVersionDTO {
	ref: string;
	version: string;
	state: ArrowState;
}

export interface ArrowListResponseItemDTO {
	namespace: string;
	name: string;
	versions: InstalledVersionDTO[];
}

export interface ApiEnvelopeDTO<T> {
	success: boolean;
	error?: string;
	data?: T;
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
	installed_constraint: string;
	user_installed: boolean;
	active_run: ActiveRun | null;
	last_return: LastReturnDTO | null;
}

export function toArrowListItems(items: ArrowListResponseItemDTO[]): ArrowListItem[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			namespace: `${arrow.namespace}@${v.ref}`,
			name: arrow.name,
			version: v.version,
			state: v.state,
			active_run: null,
			last_outcome: null,
		}))
	);
}
