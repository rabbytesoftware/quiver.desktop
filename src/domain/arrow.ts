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

/**
 * Manifest step kinds, per quiver.core's `step.StepType` (internal/domain/runtime/step/step_type.go).
 * `StepProgress.type` stays a bare `string` below -- it comes off the wire from
 * live/historical runs, and existing mock fixtures already emit values outside
 * this set (`'exec'`), so callers must not assume membership.
 */
export type StepType = 'run' | 'fetch' | 'signal' | 'dependencies';

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

/**
 * The detail endpoint's `last_return` is richer than the WebSocket's --
 * `variables`/`steps` let the UI show exactly *why* a run failed (the failed
 * step's own `error`), not just that it did. The reactive, WS-driven
 * `ArrowEntry.last_return` deliberately stays the narrower `LastReturn`
 * below: quiver.core's own runtime-update frame omits `steps` there (a
 * push on every transition carrying full step history would be wasteful),
 * so don't widen that one to match -- the richer shape is only ever
 * available from the one-time detail fetch.
 */
export interface LastReturnDetail extends LastReturn {
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface LastReturn {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
}

export interface ArrowEntry {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	version: string;
	state: ArrowState;
	active_run: ActiveRun | null;
	last_return: LastReturn | null;
}

export interface RuntimeUpdate {
	namespace: string;
	state: ArrowState;
	active_run: ActiveRun | null;
	last_return: LastReturn | null;
}

/**
 * A named credit -- quiver.core's `domain.Credit` (internal/domain/credit.go):
 * maintainers and credits are both arrays of this, never plain strings.
 */
export interface ArrowCredit {
	name: string;
	email?: string;
	url?: string;
}

export interface ArrowMedia {
	icon: string | null;
	banner: string | null;
}

/** Per-target hardware requirement -- quiver.core declares this once per OS, never once per arrow. */
export interface ArrowRequirement {
	cpu_cores: number;
	memory_gb: number;
	disk_gb: number;
}

export interface ArrowVariable {
	name: string;
	description: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	default?: string;
	values?: string[];
	min?: number;
	max?: number;
	sensitive?: boolean;
}

/** quiver.core's `netbridge.PortDef` -- name/protocol/default/required only. No live "currently bound" flag exists anywhere in core; don't invent one. */
export interface ArrowPort {
	name: string;
	protocol: 'tcp' | 'udp';
	default: number;
	required: boolean;
}

/**
 * A value that can vary per OS/arch -- quiver.core's `step.Overrideable[T]`
 * (internal/domain/runtime/step/overrideable.go). A bare scalar on the wire
 * when nothing overrides it; an object keyed by `"os/arch"` strings (plus a
 * mandatory `default`) when it does. Shown as-is rather than resolved for a
 * specific platform -- by the time a step is inside one `ArrowTarget`, it's
 * already platform-scoped, so a further override key is the rare case, not
 * the norm, and showing the raw value (whichever shape it is) is exactly
 * what "the whole raw step" means.
 */
export type Overridable<T> = T | ({ default: T } & Record<string, T>);

/** quiver.core's `step.SignalKind` (internal/domain/runtime/step/signal.go). */
export type SignalKind = 'graceful' | 'kill' | 'interrupt';

interface ArrowStepBase {
	title: string;
	/**
	 * Omitted on the wire for `dependencies` steps specifically -- core's own
	 * `DependenciesStep.MarshalJSON` never writes this field, even though it's
	 * always `true` internally for that type (internal/domain/runtime/step/dependencies.go).
	 */
	exit_on_failure?: boolean;
}

/**
 * A step declared on a not-yet-run method or lifecycle action -- the full
 * raw definition core's manifest carries per type, verified against
 * internal/domain/runtime/step/{run,fetch,signal,dependencies}.go's own
 * `MarshalJSON`. Not the runtime progress shape (`StepProgress`): this is
 * the static "what will run" declaration, before anything has.
 */
export type ArrowStepDefinition =
	| (ArrowStepBase & {
			type: 'run';
			command: Overridable<string>;
			elevated: Overridable<boolean>;
			timeout: Overridable<string>;
	  })
	| (ArrowStepBase & {
			type: 'fetch';
			url: Overridable<string>;
			to: Overridable<string>;
			checksum: Overridable<string>;
			timeout: Overridable<string>;
	  })
	| (ArrowStepBase & {
			type: 'signal';
			signal: Overridable<SignalKind>;
			timeout: Overridable<string>;
	  })
	| (ArrowStepBase & { type: 'dependencies' });

/** A manifest-declared custom method (e.g. "backup", "rcon") -- distinct from the reserved lifecycle actions in `ArrowTarget.lifecycle`. */
export interface ArrowMethod {
	name: string;
	description: string;
	available_in: ArrowState[];
	steps: ArrowStepDefinition[];
}

/** The five reserved runtime verbs, each with its own step list, separate from `methods`. */
export interface ArrowLifecycle {
	install: ArrowStepDefinition[];
	update: ArrowStepDefinition[];
	execute: ArrowStepDefinition[];
	stop: ArrowStepDefinition[];
	uninstall: ArrowStepDefinition[];
}

/** One platform's build of the arrow -- requirement, lifecycle steps, and custom methods are all per-target in core, never top-level. */
export interface ArrowTarget {
	platform: string;
	requirement: ArrowRequirement;
	lifecycle: ArrowLifecycle;
	methods: ArrowMethod[];
}

/** quiver.core's `domain.DepType` (internal/domain/dep_edge.go) -- a tool is a one-shot dependency, a service is one this arrow needs running. */
export type DependencyType = 'tool' | 'service';

/** One resolved entry from `GET /v0/arrow/:ns/dependencies` -- an arrow this one needs, namespace and ref already resolved. */
export interface ArrowDependency {
	namespace: string;
	type: DependencyType;
}

/**
 * The merged result of `GET /v0/arrow/:ns` + `GET /v0/arrow/:ns/manifest` --
 * everything the Arrow Details page needs beyond the reactive `ArrowEntry`.
 * `user_installed` is the one authoritative signal for "in the library" --
 * check it, not whether an entry happens to exist somewhere else.
 */
export interface ArrowDetail {
	namespace: string;
	name: string;
	description: string;
	license: string;
	url: string;
	tags: string[];
	media: ArrowMedia;
	maintainers: ArrowCredit[];
	credits: ArrowCredit[];
	netbridge: ArrowPort[];
	variables: ArrowVariable[];
	targets: ArrowTarget[];
	state: ArrowState;
	user_installed: boolean;
	installed_ref: string;
	installed_at?: string;
	installed_constraint?: string;
	active_run: ActiveRun | null;
	last_return: LastReturnDetail | null;
	/** Every installed ref sharing this namespace, for the version switcher -- sourced from the catalog, not this detail call. */
	versions: InstalledVersion[];
	/**
	 * The arrow's ARROW.md, raw -- full markdown, whatever an archer put there.
	 * `null` when `GET /v0/arrow/:ns/readme` 404s (quiver.core #219): the
	 * arrow was delivered as plain `arrow.yaml`, or its ARROW.md has no prose
	 * outside the fenced manifest block.
	 */
	readme: string | null;
	/** `GET /v0/arrow/:ns/dependencies` (quiver.core #220) -- what this arrow needs, resolved and topologically ordered. */
	dependencies: ArrowDependency[];
	/** `GET /v0/arrow/:ns/dependents` (quiver.core #220) -- namespace@ref of every installed arrow that declares a dependency on this one. */
	dependents: string[];
}

export interface InstalledVersion {
	ref: string;
	version: string;
	state: ArrowState;
}

/** The target whose platform matches this machine, or the first target when none matches (e.g. a package with no host-specific build). */
export function targetForPlatform(targets: ArrowTarget[], platform: string): ArrowTarget | undefined {
	return targets.find((t) => t.platform === platform) ?? targets[0];
}
