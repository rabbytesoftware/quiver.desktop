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

export interface MockPort {
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
	/** Reported by both lanes; core takes it from the vault index. */
	stars?: number;
	/** The host that served the manifest, e.g. github.com. */
	source?: string;
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

/**
 * One arrow a host advertised. Verification is a separate step: a host can list
 * an arrow whose manifest will not fetch or parse, and those count toward
 * `found` but never toward `verified`.
 */
export interface MockCandidate {
	arrow: MockArrow;
	verifiable: boolean;
}

export interface MockProvider {
	host: string;
	ok: boolean;
	returned: number;
	reason?: string;
	retry_after?: number;
}

/**
 * One discovery pass. Counts and providers stay empty until the pass ends --
 * core assigns its outcome only after the pipeline returns, so a job read
 * mid-pass reports zeroes and no providers. Results are never on the job: they
 * leave over the socket and nowhere else.
 */
export interface MockDiscoveryJob {
	id: string;
	status: 'running' | 'completed';
	query: string;
	expires_at: string;
	found: number;
	verified: number;
	skipped: number;
	providers: MockProvider[];
}

export interface Clock {
	after(ms: number, fn: () => void): void;
	every(ms: number, fn: () => void): () => void;
	cancelAll(): void;
}

export interface Emitter {
	emit(endpoint: string, frame: unknown): void;
}

// The daemon config document, keyed by section then setting name. Same shape
// on the wire regardless of scenario, so — unlike arrows/collections — it is
// not part of a ScenarioDataset.
export interface MockConfigDoc {
	[section: string]: Record<string, unknown>;
}

export const CONFIG_DEFAULTS: MockConfigDoc = {
	netbridge: { enabled: true, ephemeral_port_start: 49152, ephemeral_port_end: 65535 },
	api: { host: 'unix://' },
	logger: { enabled: true, level: 'info' },
	manifold: { fetch_timeout: '30s' },
	vault: { sweep_interval: '5m', ttl: '24h', index_ttl: '24h' },
	arrows: { auto_retry: { enabled: true, retries: 3 } },
	search: { per_provider_limit: 25, fetch_concurrency: 4, provider_timeout: '10s' },
};

export interface MockConfigState {
	// What the daemon booted with; only a restart moves this.
	running: MockConfigDoc;
	// What the daemon's next start will use; PATCH /v0/config writes here.
	configured: MockConfigDoc;
	// Damage already in the on-disk file, which only the daemon can see.
	corrected: { key: string; message: string }[];
}

export interface MockWorld {
	scenario: ScenarioName;
	connectionId: string;
	/** The catalog: what this machine has. Keyed by `namespace@ref`. */
	arrows: Map<string, MockArrow>;
	/**
	 * Arrows that exist on a host but not in the catalog -- what a pass can
	 * find. Keeping them out of `arrows` is the point: a discovery result whose
	 * arrow is already installed can never exercise the merge.
	 */
	discoverable: MockCandidate[];
	/**
	 * Bare namespaces whose manifest the vault index has cached. Discovery
	 * writes here before emitting, so a rediscovered arrow comes back known but
	 * not installed -- browsing an arrow is not having it.
	 */
	vault: Set<string>;
	collections: Map<string, MockCollection>;
	jobs: Map<string, MockDiscoveryJob>;
	cancels: Map<string, () => void>;
	clock: Clock;
	emitter: Emitter;
	config: MockConfigState;

	nextId: () => number;
}

export function versioned(arrow: Pick<MockArrow, 'namespace' | 'ref'>): string {
	return `${arrow.namespace}@${arrow.ref}`;
}
