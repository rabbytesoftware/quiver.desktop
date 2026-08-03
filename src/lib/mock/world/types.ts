// What the mock daemon knows. Deliberately richer than any single DTO: the
// handlers project this down to whatever shape each route returns, the same way
// quiver.core projects its domain onto `internal/api/v0/dto`. Modelling the
// DTOs directly would mean holding the same arrow four times and keeping the
// copies in step by hand.

import type { ActiveRun, ArrowState, StepProgress } from '@/domain/arrow';

export type ScenarioName = 'normal' | 'extreme' | 'empty';

export const SCENARIO_NAMES: ScenarioName[] = ['normal', 'extreme', 'empty'];

/**
 * The platform this fake daemon claims to run on.
 *
 * FIXED, not read from the real host. An arrow with no target for your platform
 * is one of the failure screens this scenario exists to reach, and it can only
 * be reached reliably if "your platform" is the same everywhere — otherwise the
 * screen appears on macOS and silently vanishes on the Linux CI box.
 */
export const MOCK_HOST_PLATFORM = 'darwin/arm64';

/** Mirrors quiver.core's `Variable`. `type` drives which control renders. */
export interface MockVariable {
	name: string;
	description: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	default?: string;
	values?: string[];
	min?: number;
	max?: number;
	/**
	 * A DISPLAY hint and nothing more, exactly as in core: the value still comes
	 * back in `last_return.variables` as plaintext. The UI masks the input; it
	 * must never claim the value is protected.
	 */
	sensitive?: boolean;
}

/** Mirrors `PortDef` in an arrow's `netbridge` block. */
export interface MockPort {
	name: string;
	protocol: 'tcp' | 'udp';
	default: number;
	required: boolean;
}

/**
 * Mirrors `Method`. `available_in` is what decides whether a method's button is
 * live: core rejects an execute whose method does not list the arrow's current
 * state, so the UI must not offer it.
 */
export interface MockMethod {
	name: string;
	description: string;
	available_in: Array<'ready' | 'running'>;
	/** Step titles, in order. The clock walks these to fabricate progress. */
	steps: string[];
}

export interface MockTarget {
	/** `os/arch`, e.g. `darwin/arm64`. */
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
	/** UNVERSIONED base, e.g. `github.com/rabbyte/valheim`. */
	namespace: string;
	/** The ref this entry is pinned to. `${namespace}@${ref}` is the store key. */
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
	/**
	 * Whether this arrow is in the user's LIBRARY. `GET /v0/arrow` filters on it;
	 * everything else in the map is discoverable but not added, which is the
	 * only way search can return something the library does not already hold.
	 */
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
	/** Versioned namespace of the member arrow. */
	namespace: string;
	/**
	 * Whether this member resolves to an arrow the daemon can see. An unresolved
	 * member is not an error — a collection may name an arrow whose host is down
	 * or whose ref was yanked — and the detail screen has to say which.
	 */
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

/** One host's answer during a discovery pass. Mirrors `DiscoveryProviderDTO`. */
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
	/** Versioned namespaces this pass turned up. */
	results: string[];
}

/**
 * Cancellable timers, owned in one place.
 *
 * Every fabricated transition is a timer, and a scenario switch, an uninstall
 * mid-install, or a page teardown must not leave one running against a world
 * nobody is reading any more — it would go on emitting frames into a hub whose
 * sockets have closed, and in tests it would leak across cases.
 */
export interface Clock {
	after(ms: number, fn: () => void): void;
	every(ms: number, fn: () => void): () => void;
	cancelAll(): void;
}

/** Where fabricated frames go. Implemented by the socket hub. */
export interface Emitter {
	emit(endpoint: string, frame: unknown): void;
}

export interface MockWorld {
	scenario: ScenarioName;
	/** Cache partition. `mock:<scenario>`, so no two scenarios and no real host can collide. */
	connectionId: string;
	/** Key: VERSIONED namespace (`base@ref`). */
	arrows: Map<string, MockArrow>;
	collections: Map<string, MockCollection>;
	jobs: Map<string, MockDiscoveryJob>;
	/**
	 * How to stop the run in flight for a versioned namespace.
	 *
	 * An install is a repeating timer, and an uninstall landing mid-install has
	 * to end it — otherwise the install goes on ticking and writes `ready` over
	 * the uninstall that superseded it, which looks exactly like the app
	 * ignoring the button you just pressed.
	 */
	cancels: Map<string, () => void>;
	clock: Clock;
	emitter: Emitter;
	/** Monotonic, for fabricating ids without a random source. */
	nextId: () => number;
}

/** `${namespace}@${ref}` — the key everything above the DTO layer uses. */
export function versioned(arrow: Pick<MockArrow, 'namespace' | 'ref'>): string {
	return `${arrow.namespace}@${arrow.ref}`;
}
