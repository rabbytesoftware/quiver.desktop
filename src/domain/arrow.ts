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

export interface LastReturn {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
}

/** Merged catalog + runtime view. Key: versioned namespace (base@ref). */
export interface ArrowEntry {
	namespace:   string;
	name:        string;
	description: string;
	tags:        string[];
	icon:        string | null;
	banner:      string | null;
	version:     string;
	state:       ArrowState;
	active_run:  ActiveRun | null;
	last_return: LastReturn | null;
}

export interface RuntimeUpdate {
	namespace:   string;
	state:       ArrowState;
	active_run:  ActiveRun | null;
	last_return: LastReturn | null;
}

/** @deprecated Use ArrowEntry */
export type ArrowListItem = ArrowEntry;

/** @deprecated Use LastReturn */
export type LastOutcome = LastReturn;
