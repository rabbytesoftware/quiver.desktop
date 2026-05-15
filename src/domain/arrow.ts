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

/** The global store entry — merged catalog + runtime view. Key: versioned namespace (base@ref). */
export interface ArrowListItem {
	namespace: string;
	name: string;
	version: string;
	state: ArrowState;
	active_run: ActiveRun | null;
	last_outcome: LastOutcome | null;
}

export interface RuntimeUpdate {
	namespace: string;
	state: ArrowState;
	active_run: ActiveRun | null;
	last_outcome: LastOutcome | null;
}
