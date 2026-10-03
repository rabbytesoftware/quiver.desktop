import type {
	ActiveRun,
	ArrowState,
	ArrowStepDefinition,
	AvailableVersion,
	PendingActivation,
	SelectorKind,
	StepProgress,
} from '@/domain/arrow';
import { bareNamespace } from '@/lib/namespace';

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
	/** Any `ArrowState` -- core lets a manifest gate a custom method into any of them, not just ready/running. */
	available_in: ArrowState[];
	steps: ArrowStepDefinition[];
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

/** Mirrors `ArrowChannel`/`ChannelDTO` -- one published release channel, as `GET /v0/arrow/:ns/channels` reports it. */
export interface MockChannel {
	name: string;
	kind: 'ordered' | 'pointer';
	latest: string;
	count?: number;
	members?: string[];
}

export interface MockArrow {
	namespace: string;
	/** The identity selector the row is filed under -- `namespace@ref` is its key, and it never changes. */
	ref: string;
	version: string;
	name: string;
	description: string;
	license: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	maintainers: string[];
	/** Optional -- most fixtures have none; `arrow()` defaults it to `[]`. */
	credits?: string[];
	url: string;
	user_installed: boolean;
	state: ArrowState;
	installed_at: string;
	/** When `execute` last completed successfully -- mirrors quiver.core's still-unmerged `last_used_at` (enhancement/last_used). Undefined for an arrow that's never been run. */
	last_used_at?: string;
	requirement: MockRequirement;
	netbridge: MockPort[];
	variables: MockVariable[];
	targets: MockTarget[];
	active_run: ActiveRun | null;
	last_return: MockLastReturn | null;
	/** Undefined reads as core classifies it: a channel when `ref` names one of `channels`, a pin otherwise. See `selectorKindOf`. */
	selector_kind?: SelectorKind;
	/** The ref the row resolved to. Undefined reads as `ref` itself, or a channel's `latest`. See `resolvedRefOf`. */
	resolved_ref?: string;
	/** What is ahead of `resolved_ref`, undefined when current. `arrow()` gives every `outdated` fixture one. */
	available?: AvailableVersion;
	/** An update that finishes by staging for a restart, as quiver.core's own does, instead of advancing the row. */
	stages_update?: boolean;
	/** A staged update waiting for `activate`. */
	pending_activation?: PendingActivation;
	/** Every channel this arrow's repo publishes -- `GET /v0/arrow/:ns/channels`. Undefined (not `[]`) for a fixture that publishes none, same convention as `dependencies` below. */
	channels?: MockChannel[];
	/** Reported by both lanes; core takes it from the vault index. */
	stars?: number;
	/** The host that served the manifest, e.g. github.com. */
	source?: string;
	/** Undefined for a declared arrow. `inferred` means Fletcher built the manifest, with `confidence` alongside. */
	origin?: 'declared' | 'inferred';
	confidence?: 'high' | 'medium' | 'low';
	/** Raw markdown. Undefined for arrows without an ARROW.md -- Overview then falls back to Details. See `ArrowDetail.readme`'s own comment for the wire status. */
	readme?: string;
	/**
	 * What this arrow needs, namespace@ref already resolved -- the mock's
	 * stand-in for quiver.core's resolved dependency plan (quiver.core #220).
	 * `dependents` (the reverse direction) is never stored -- it's derived by
	 * scanning every arrow's own `dependencies` for one pointing back here,
	 * same as `toArrowDependentsDTO` does for the real endpoint.
	 */
	dependencies?: { namespace: string; type: 'tool' | 'service' }[];
}

export interface MockCollectionMember {
	namespace: string;
	resolved: boolean;
	name?: string;
	description?: string;
}

export interface MockCollectionMedia {
	icon?: string;
	banner?: string;
}

export interface MockCollection {
	namespace: string;
	name: string;
	description: string;
	maintainers: string[];
	followed: boolean;
	url?: string;
	tags: string[];
	media?: MockCollectionMedia;
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
	/** Ends a stream the way the daemon does, with a close code and reason. */
	close?(endpoint: string, code: number, reason: string): void;
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
	manifold: { fetch_timeout: '30s', fletcher: { enabled: false } },
	vault: { sweep_interval: '5m', ttl: '24h', index_ttl: '24h' },
	arrows: { auto_retry: { enabled: true, retries: 3 }, self_update_channel: '' },
	search: {
		per_provider_limit: 25,
		fetch_concurrency: 4,
		provider_timeout: '10s',
		unmarked: { min_stars: 50, probe_limit: 10 },
	},
};

export interface MockConfigState {
	// What the daemon booted with; only a restart moves this.
	running: MockConfigDoc;
	// What the daemon's next start will use; PATCH /v0/config writes here.
	configured: MockConfigDoc;
	// Damage already in the on-disk file, which only the daemon can see.
	corrected: { key: string; message: string }[];
}

export interface MockPathState {
	bin_dir: string;
	on_path: boolean;
	configured: boolean;
	files: string[];
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
	path: MockPathState;

	nextId: () => number;
}

export function versioned(arrow: Pick<MockArrow, 'namespace' | 'ref'>): string {
	return `${arrow.namespace}@${arrow.ref}`;
}

/**
 * `world.arrows` is keyed by the ref-qualified `namespace@ref` (see
 * `versioned`), since that's the only key that stays unambiguous once an
 * arrow has more than one installed version. Real quiver.core's `:ns` route
 * param accepts a bare namespace too, though -- Search links to a Discovered
 * or single-version arrow with just its namespace, having no installed ref to
 * offer. Fall back to a scan for the one arrow whose bare namespace matches
 * when an exact key lookup misses, so a bare-namespace request still resolves
 * for any arrow that isn't genuinely ambiguous between multiple versions.
 */
export function findArrow(arrows: Map<string, MockArrow>, ns: string): MockArrow | undefined {
	const exact = arrows.get(ns);
	if (exact) return exact;
	return [...arrows.values()].find((a) => a.namespace === ns);
}

/** Any row of the repository `ns` names, whatever selector it follows -- what a registration of a new selector is modelled on. */
export function findRepository(arrows: Map<string, MockArrow>, ns: string): MockArrow | undefined {
	const bare = bareNamespace(ns);
	return [...arrows.values()].find((a) => a.namespace === bare);
}

/** Mirrors the order of core's `ClassifySelector`: a channel name, then a glob, then a bare commit, else a pin. */
export function classifySelector(selector: string, channels: MockChannel[] = []): SelectorKind {
	if (channels.some((c) => c.name === selector)) return 'channel';
	if (/[*?[]/.test(selector)) return 'constraint';
	if (/^[0-9a-f]{7,40}$/.test(selector)) return 'commit';
	return 'pin';
}

export function selectorKindOf(arrow: MockArrow): SelectorKind {
	return arrow.selector_kind ?? classifySelector(arrow.ref, arrow.channels);
}

export function resolvedRefOf(arrow: MockArrow): string {
	if (arrow.resolved_ref !== undefined) return arrow.resolved_ref;
	const channel = (arrow.channels ?? []).find((c) => c.name === arrow.ref);
	return channel?.latest ?? arrow.ref;
}
