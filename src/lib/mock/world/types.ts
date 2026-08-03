import type { ActiveRun, ArrowState, StepProgress } from '@/domain/arrow';

export type ScenarioName = 'normal' | 'extreme' | 'empty';

export const SCENARIO_NAMES: ScenarioName[] = ['normal', 'extreme', 'empty'];

/** Fixed, not read from the real host, so the no-target-for-your-platform
 *  screen appears identically on every machine. */
export const MOCK_HOST_PLATFORM = 'darwin/arm64';

/** Mirrors quiver.core's `Variable`. */
export interface MockVariable {
	name: string;
	description: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	default?: string;
	values?: string[];
	min?: number;
	max?: number;
	/** A display hint only — core still returns the value in
	 *  `last_return.variables` as plaintext. */
	sensitive?: boolean;
}

export interface MockPort {
	name: string;
	protocol: 'tcp' | 'udp';
	default: number;
	required: boolean;
}

/** `available_in` decides whether a method may be offered: core rejects an
 *  execute whose method does not list the arrow's current state. */
export interface MockMethod {
	name: string;
	description: string;
	available_in: Array<'ready' | 'running'>;
	/** Step titles, in order. */
	steps: string[];
}

export interface MockTarget {
	platform: string;
	methods: Record<string, MockMethod>;
}

export interface MockRequirement {
	cpu_cores: number;
	memory_gb: number;
	disk_gb: number;
}

export interface MockLastReturn {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface MockArrow {
	/** Unversioned base. `${namespace}@${ref}` is the store key. */
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
	/** In the user's library. Everything else is discoverable but not added,
	 *  which is how search can return something the rail does not show. */
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
	/** An unresolved member is not an error — the collection is fine and the
	 *  member is not — so the row is kept and flagged rather than dropped. */
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

/** Mirrors `DiscoveryProviderDTO`. */
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

/** Cancellable timers, owned in one place. See `createClock`. */
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
	/** Cache partition: `mock:<scenario>`. */
	connectionId: string;
	/** Key: versioned namespace. */
	arrows: Map<string, MockArrow>;
	collections: Map<string, MockCollection>;
	jobs: Map<string, MockDiscoveryJob>;
	/** How to stop the run in flight for a versioned namespace. */
	cancels: Map<string, () => void>;
	clock: Clock;
	emitter: Emitter;

	nextId: () => number;
}

export function versioned(arrow: Pick<MockArrow, 'namespace' | 'ref'>): string {
	return `${arrow.namespace}@${arrow.ref}`;
}
