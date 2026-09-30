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
export type StepType = 'run' | 'fetch' | 'extract' | 'portable' | 'signal' | 'dependencies' | 'expose' | 'unexpose';

export type StepStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface StepProgress {
	index: number;
	title: string;
	status: StepStatus;
	type: string;
	error?: string;
	/** Why a completed step did nothing (e.g. an `auto` expose that found nothing). Not a failure. */
	note?: string;
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

/**
 * Where an arrow's manifest came from. `declared`: the repository ships an
 * `ARROW.md` / `arrow.yaml`. `inferred`: quiver.core's Fletcher built one from
 * the repository's release downloads.
 */
export type ArrowOrigin = 'declared' | 'inferred';

/** How sure Fletcher was of an inferred manifest. Only ever set alongside `origin: 'inferred'`. */
export type InferenceConfidence = 'high' | 'medium' | 'low';

/**
 * The two fields every arrow-shaped record carries once quiver.core reports
 * how the manifest was made. Both are optional so a record built by hand (a
 * fixture, an older cache row) reads as declared, which is what absent means.
 */
export interface ArrowOriginFields {
	origin?: ArrowOrigin;
	confidence?: InferenceConfidence | null;
}

/**
 * Narrows quiver.core's `origin` string. An older daemon that sends none, or a
 * newer one that sends a value this build does not know, reads as a declared
 * arrow rather than a broken one.
 */
export function parseArrowOrigin(value: string | undefined | null): ArrowOrigin {
	return value === 'inferred' ? 'inferred' : 'declared';
}

const CONFIDENCES: readonly InferenceConfidence[] = ['high', 'medium', 'low'];

export function parseInferenceConfidence(value: string | undefined | null): InferenceConfidence | null {
	return CONFIDENCES.find((confidence) => confidence === value) ?? null;
}

/** Only an explicit `inferred` counts: anything else, including absent, is a declared manifest. */
export function isInferred(arrow: ArrowOriginFields): boolean {
	return arrow.origin === 'inferred';
}

export interface ArrowEntry extends ArrowOriginFields {
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
	/**
	 * When this arrow's `execute` last completed successfully. Optional because
	 * quiver.core doesn't stamp it yet (tracked separately, enhancement/last_used) --
	 * every arrow reports `undefined`/`null` until that lands, which is exactly
	 * the "never used" case Home's Recents section already treats as absent.
	 */
	last_used_at?: string | null;
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
	| (ArrowStepBase & { type: 'dependencies' })
	| (ArrowStepBase & { type: 'extract' | 'portable' | 'expose' | 'unexpose' });

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

/**
 * One published release channel -- `GET /v0/arrow/:ns/channels`. `ordered`
 * carries ranked tags (`members`, highest-precedence first, `members[0]`
 * equal to `latest`); `pointer` is an unversioned branch/ref with nothing to
 * rank, so `count`/`members` are absent and `latest` is its only version.
 */
export interface ArrowChannel {
	name: string;
	kind: 'ordered' | 'pointer';
	latest: string;
	count?: number;
	members?: string[];
}

/**
 * How a catalog identity's selector is followed, as quiver.core classified it
 * once when the row was created: a channel's newest member, the highest tag a
 * constraint matches, exactly one ref, or exactly one commit.
 */
export type SelectorKind = 'pin' | 'channel' | 'constraint' | 'commit';

const SELECTOR_KINDS: readonly SelectorKind[] = ['pin', 'channel', 'constraint', 'commit'];

/** An older core sends no kind at all, which quiver.core itself reads as a pin of the identity's ref. */
export function parseSelectorKind(value: string | undefined | null): SelectorKind {
	return SELECTOR_KINDS.find((kind) => kind === value) ?? 'pin';
}

/** A newer ref an installed row's selector points at, and the commit it resolves to. */
export interface AvailableVersion {
	ref: string;
	commit: string;
}

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
export interface ArrowDetail extends ArrowOriginFields {
	/** Machine codes from an inferred manifest's generator, e.g. `assumed_arch`; empty for a declared arrow. */
	warnings?: string[];
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
	installed_at?: string;
	/**
	 * The selector this catalog row follows -- the part of `namespace` after
	 * its first `@`. It never changes for the life of the row: following
	 * something else is a different row (uninstall, then install that one).
	 */
	selector: string;
	selector_kind: SelectorKind;
	/** The ref this row resolved to -- what is installed, or what an install would put on disk. Empty until one is resolved. */
	resolved_ref: string;
	installed_commit: string;
	/** What the last version check found ahead of `resolved_ref`, or `null` when current. */
	available: AvailableVersion | null;
	/** True exactly when `available` is set. */
	outdated: boolean;
	active_run: ActiveRun | null;
	last_return: LastReturnDetail | null;
	/**
	 * Every channel this arrow's repo publishes -- `GET /v0/arrow/:ns/channels`.
	 * Always an array, empty when there is nothing to show. Answers "what could
	 * a new identity follow", not "what does this one follow" -- see `selector`.
	 */
	channels: ArrowChannel[];
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

/** The target whose platform matches this machine, or the first target when none matches (e.g. a package with no host-specific build). */
export function targetForPlatform(targets: ArrowTarget[], platform: string): ArrowTarget | undefined {
	return targets.find((t) => t.platform === platform) ?? targets[0];
}

/**
 * Whether this arrow genuinely publishes a build for `platform` -- an exact
 * match, no fallback. Deliberately separate from `targetForPlatform`: that
 * one's fallback to `targets[0]` exists for manifests with a single universal
 * target and must keep returning something for its own callers (the Methods
 * rail, `computeActions`). This is the only place that can tell "no match"
 * from "one universal target", which is exactly what a not-supported warning
 * needs to know.
 */
export function isPlatformSupported(targets: ArrowTarget[], platform: string): boolean {
	return targets.some((t) => t.platform === platform);
}
