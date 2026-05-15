export type ArrowState =
	| 'absent'
	| 'installing'
	| 'updating'
	| 'ready'
	| 'running'
	| 'stopping'
	| 'draining'
	| 'detached'
	| 'uninstalling'
	| 'removed'
	| 'outdated';

export type StepStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface StepProgress {
	index: number;
	title: string;
	status: StepStatus;
	type: string;
	error?: string;
}

export interface ActiveRun {
	method: string;
	pid?: number;
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface LastOutcome {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
}

export interface LastReturn {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

/** The global store entry — merged catalog + runtime view. Key: versioned namespace (base@ref). */
export interface ArrowListItem {
	namespace: string;
	name: string;
	version: string;
	state: ArrowState;
	active_run: ActiveRun | null;
	last_outcome: LastOutcome | null;
}

export interface InstalledVersion {
	ref: string;
	version: string;
	state: ArrowState;
	installed_at: string;
	constraint?: string;
}

/** Shape of GET /arrow list response items — used for initial hydration. */
export interface ArrowListResponse {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	versions: InstalledVersion[];
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
	last_return: LastReturn | null;
}
