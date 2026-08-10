import type { ActiveRun, ArrowState, StepProgress } from '@/domain/arrow';

export type ScenarioName = 'normal' | 'extreme' | 'empty';

export const SCENARIO_NAMES: ScenarioName[] = ['normal', 'extreme', 'empty'];

export const MOCK_HOST_PLATFORM = 'darwin/arm64';

export interface MockVariable {
	name: string;
	description: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	default?: string;
	values?: string[];
	min?: number;
	max?: number;
	sensitive?: boolean;
}

interface MockPort {
	name: string;
	protocol: 'tcp' | 'udp';
	default: number;
	required: boolean;
}

export interface MockMethod {
	name: string;
	description: string;
	available_in: Array<'ready' | 'running'>;
	steps: string[];
}

export interface MockTarget {
	platform: string;
	methods: Record<string, MockMethod>;
}

interface MockRequirement {
	cpu_cores: number;
	memory_gb: number;
	disk_gb: number;
}

interface MockLastReturn {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface MockArrow {
	namespace: string;
	ref: string;
	version: string;
	name: string;
	description: string;
	license: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	maintainers: string[];
	url: string;
	user_installed: boolean;
	state: ArrowState;
	installed_at: string;
	requirement: MockRequirement;
	netbridge: MockPort[];
	variables: MockVariable[];
	targets: MockTarget[];
	active_run: ActiveRun | null;
	last_return: MockLastReturn | null;
}

export interface MockCollectionMember {
	namespace: string;
	resolved: boolean;
	reason?: string;
}

export interface MockCollection {
	namespace: string;
	name: string;
	description: string;
	maintainers: string[];
	followed: boolean;
	arrows: MockCollectionMember[];
}

export interface MockProvider {
	host: string;
	ok: boolean;
	returned: number;
	reason?: string;
	retry_after?: number;
}

export interface MockDiscoveryJob {
	id: string;
	status: 'running' | 'done';
	query: string;
	providers: MockProvider[];
	results: string[];
}

export interface Clock {
	after(ms: number, fn: () => void): void;
	every(ms: number, fn: () => void): () => void;
	cancelAll(): void;
}

export interface Emitter {
	emit(endpoint: string, frame: unknown): void;
}

export interface MockWorld {
	scenario: ScenarioName;
	connectionId: string;
	arrows: Map<string, MockArrow>;
	collections: Map<string, MockCollection>;
	jobs: Map<string, MockDiscoveryJob>;
	cancels: Map<string, () => void>;
	clock: Clock;
	emitter: Emitter;

	nextId: () => number;
}

export function versioned(arrow: Pick<MockArrow, 'namespace' | 'ref'>): string {
	return `${arrow.namespace}@${arrow.ref}`;
}
