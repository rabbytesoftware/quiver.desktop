import type {
	ActiveRun,
	ArrowChannel,
	ArrowCredit,
	ArrowDependency,
	ArrowDetail,
	ArrowLifecycle,
	ArrowMethod,
	ArrowState,
	ArrowStepDefinition,
	ArrowTarget,
	AvailableVersion,
	DependencyType,
	RuntimeUpdate,
	StepProgress,
} from '@/domain/arrow';
import { parseArrowOrigin, parseInferenceConfidence, parseSelectorKind } from '@/domain/arrow';
import { selectorOf } from '@/lib/namespace';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

export interface InferenceDTO {
	generator?: string;
	confidence?: string;
	warnings?: string[];
}

/**
 * One catalog row of an arrow. `ref` is the identity selector the row is filed
 * under (`stable`, `v1.*`, `v1.2.0`, a commit) and never changes; `resolved_ref`
 * is the ref it resolved to, empty until it resolved one. Absent altogether on
 * an older core, which reads the same as unresolved.
 */
export interface InstalledVersionDTO {
	ref: string;
	resolved_ref?: string;
	state: ArrowState;
	/** Omitted until a successful install stamps it, and again after an uninstall. */
	installed_at?: string;
	/** Set once quiver.core stamps it on a completed `execute` (enhancement/last_used); absent until then. */
	last_used_at?: string;
}

export interface ArrowListResponseItemDTO {
	namespace: string;
	name: string;
	description: string;
	/**
	 * NULL, not absent and not empty, for any arrow whose manifest declares no
	 * `tags:` -- which includes both of Quiver's own self-arrows. Go marshals
	 * a nil slice as JSON null, and quiver.core's DTO carries the slice
	 * through untouched. Every mapper below has to default it; a consumer
	 * reading `.length` off what this hands back crashes the page otherwise.
	 */
	tags: string[] | null;
	media?: {
		icon?: string | null;
		banner?: string | null;
	};
	versions: InstalledVersionDTO[];
	/** Absent on a daemon that predates inference; reads as declared. */
	origin?: string;
	/** Omitted unless the arrow is inferred. */
	confidence?: string;
}

export interface LastReturnDTO {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface AvailableDTO {
	ref: string;
	commit: string;
}

/**
 * `GET /v0/arrow/:ns`. `namespace` is the catalog identity, `bare@selector`,
 * always -- an uncatalogued refless read is resolved live and reported under
 * the repository's default channel. The row-state fields are optional because
 * an older core sends none of them; the mapper reads that as a pin with
 * nothing resolved.
 */
export interface ArrowDetailDTO {
	namespace: string;
	name: string;
	description: string;
	license?: string;
	state: ArrowState;
	/** Null for an arrow with no `tags:`, exactly as above. */
	tags: string[] | null;
	/** Omitted while the arrow is not on disk. */
	installed_at?: string;
	last_used_at?: string;
	user_installed: boolean;
	selector_kind?: 'pin' | 'channel' | 'constraint' | 'commit';
	resolved_ref?: string;
	installed_commit?: string;
	/** Omitted when current. */
	available?: AvailableDTO | null;
	outdated?: boolean;
	active_run?: ActiveRun | null;
	last_return?: LastReturnDTO | null;
	/** Absent on a daemon that predates inference; reads as declared. */
	origin?: string;
	/** Omitted unless the arrow is inferred. */
	inference?: InferenceDTO;
}

/**
 * `GET /v0/arrow/:ns/channels` -- every release channel this arrow's repo
 * publishes. `kind` is `"ordered"` (ranked tags) or `"pointer"` (an
 * unversioned branch/ref); `count`/`members` are `omitempty` on the Go side
 * and present only for `kind: "ordered"`. `members` is already sorted
 * highest-precedence first, `members[0]` equal to `latest`.
 */
export interface ChannelDTO {
	name: string;
	kind: 'ordered' | 'pointer';
	latest: string;
	count?: number;
	members?: string[];
}

export interface ChannelListDTO {
	channels: ChannelDTO[];
}

export function toArrowChannels(dto: ChannelListDTO): ArrowChannel[] {
	return dto.channels.map((c) => ({
		name: c.name,
		kind: c.kind,
		latest: c.latest,
		count: c.count,
		members: c.members,
	}));
}

export function toArrowCatalogRecords(items: ArrowListResponseItemDTO[], connectionId: string): ArrowCatalogRecord[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			connectionId,
			namespace: `${arrow.namespace}@${v.ref}`,
			name: arrow.name,
			description: arrow.description,
			tags: arrow.tags ?? [],
			icon: arrow.media?.icon || null,
			banner: arrow.media?.banner || null,
			version: v.resolved_ref ?? '',
			last_used_at: v.last_used_at ?? null,
			origin: parseArrowOrigin(arrow.origin),
			confidence: parseInferenceConfidence(arrow.confidence),
		}))
	);
}

export function toInitialRuntimeUpdates(items: ArrowListResponseItemDTO[]): RuntimeUpdate[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			namespace: `${arrow.namespace}@${v.ref}`,
			state: v.state,
			active_run: null,
			last_return: null,
		}))
	);
}

/**
 * `GET /v0/arrow/:ns/manifest` -- everything `ArrowDetailDTO` above deliberately
 * lacks. Two real endpoints, not one: quiver.core never returns media,
 * maintainers, credits, url, requirements, netbridge, variables, or methods
 * from the plain detail call (verified against `internal/api/v0/dto/arrow_detail.go`).
 */
export interface CreditDTO {
	name: string;
	email?: string;
	url?: string;
}

export interface ArrowMediaDTO {
	icon?: string;
	banner?: string;
}

/** Identical to the domain shape -- core's wire step is already the full raw definition, nothing to rename or reshape on the way in. */
export type StepDTO = ArrowStepDefinition;

export interface LifecycleDTO {
	install: StepDTO[];
	update: StepDTO[];
	execute: StepDTO[];
	stop: StepDTO[];
	uninstall: StepDTO[];
}

export interface MethodDTO {
	name: string;
	description: string;
	available_in: ArrowState[];
	steps: StepDTO[];
}

export interface TargetManifestDTO {
	requirements: { cpu_cores: number; memory_gb: number; disk_gb: number };
	lifecycle: LifecycleDTO;
	methods: Record<string, MethodDTO>;
}

export interface VariableDTO {
	name: string;
	description: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	default?: string;
	values?: string[];
	min?: number;
	max?: number;
	sensitive?: boolean;
}

export interface PortDTO {
	name: string;
	protocol: 'tcp' | 'udp';
	default: number;
	required: boolean;
}

/**
 * The manifest's `metadata:` block as its author wrote it. Every list here can
 * arrive as JSON null rather than absent or empty: Go marshals a nil slice
 * that way and quiver.core carries the manifest through untouched. `tags`
 * arriving null once crashed the whole arrow-details view on
 * `detail.tags.length`, so every list is defaulted at this boundary.
 */
export interface ManifestMetadataDTO {
	name?: string;
	description?: string;
	license?: string;
	url?: string;
	maintainers?: CreditDTO[] | null;
	credits?: CreditDTO[] | null;
	media?: ArrowMediaDTO | null;
	tags?: string[] | null;
}

/** The manifest as its author wrote it -- the only place url/maintainers/credits/media/netbridge live on the wire. Row state (selector, resolved and available refs) is never here. */
export interface ManifestContentDTO {
	metadata: ManifestMetadataDTO | null;
	variables: VariableDTO[] | null;
	/** Neither of Quiver's own self-manifests declares `netbridge:`, so this is null for both. */
	netbridge: PortDTO[] | null;
	targets: Record<string, TargetManifestDTO> | null;
	readme?: string;
}

export interface ArrowManifestDTO {
	namespace: string;
	name: string;
	description: string;
	tags: string[] | null;
	variables: VariableDTO[] | null;
	targets: Record<string, TargetManifestDTO>;
	/** A pointer on the Go side, so null when core had no manifest to report. */
	manifest: ManifestContentDTO | null;
}

/**
 * `GET /v0/arrow/:ns/readme` -- a third, separate endpoint (quiver.core PR #219),
 * not a field on the manifest. `:ns` must be a bare namespace; core rejects a
 * `namespace@ref` path here the same way it already does for `/manifest`
 * (`ErrInvalidNamespace`, 400). 404s when the arrow has no ARROW.md prose
 * (plain `arrow.yaml`, or an ARROW.md with nothing outside its fenced block).
 */
export interface ArrowReadmeDTO {
	namespace: string;
	readme: string;
}

/**
 * `GET /v0/arrow/:ns/dependencies` (quiver.core PR #220) -- the resolved,
 * topologically ordered dependency plan for this arrow, from its manifest's
 * declared tools/services. `:ns` is the full `namespace@ref`: dependency
 * resolution is version-specific, unlike `/manifest` and `/readme`.
 */
export interface ArrowDependencyDTO {
	namespace: string;
	type: string;
}

export interface ArrowDependenciesDTO {
	namespace: string;
	dependencies: ArrowDependencyDTO[];
}

/**
 * `GET /v0/arrow/:ns/dependents` (quiver.core PR #220) -- namespace@ref of
 * every installed arrow that declares a dependency on this one. `:ns` also
 * takes the full `namespace@ref`, though core normalizes to bare internally.
 */
export interface ArrowDependentsDTO {
	namespace: string;
	dependents: string[];
}

function toCredit(dto: CreditDTO): ArrowCredit {
	return { name: dto.name, email: dto.email, url: dto.url };
}

// `type` comes off the wire as `domain.DepType`'s two real values ("tool" |
// "service") -- cast, not validated, matching how `state`/`outcome` etc.
// are trusted as-is everywhere else in this file.
function toDependency(dto: ArrowDependencyDTO): ArrowDependency {
	return { namespace: dto.namespace, type: dto.type as DependencyType };
}

/** Exported (unlike `toDependency` above) so `queries/arrow.ts`'s own `useArrowDependencies` can map the raw fetch result without reaching into this file's private helpers. */
export function toArrowDependencies(dtos: ArrowDependencyDTO[]): ArrowDependency[] {
	return (dtos ?? []).map(toDependency);
}

// `StepDTO` is `ArrowStepDefinition` verbatim (see its declaration above), so
// the lifecycle/method step lists need no per-step mapping -- unlike every
// other DTO here, there's nothing to rename or reshape.
function toLifecycle(dto: LifecycleDTO): ArrowLifecycle {
	return {
		install: dto.install,
		update: dto.update,
		execute: dto.execute,
		stop: dto.stop,
		uninstall: dto.uninstall,
	};
}

function toMethod(dto: MethodDTO): ArrowMethod {
	return {
		name: dto.name,
		description: dto.description,
		available_in: dto.available_in,
		steps: dto.steps,
	};
}

function toTargets(dto: ArrowManifestDTO['targets']): ArrowTarget[] {
	return Object.entries(dto).map(([platform, target]) => ({
		platform,
		requirement: target.requirements,
		lifecycle: toLifecycle(target.lifecycle),
		methods: Object.values(target.methods).map(toMethod),
	}));
}

function toAvailable(dto: AvailableDTO | null | undefined): AvailableVersion | null {
	return dto?.ref ? { ref: dto.ref, commit: dto.commit ?? '' } : null;
}

/**
 * Merges the six real endpoints into the one shape the Arrow Details page
 * consumes. `channels` comes from `GET /v0/arrow/:ns/channels` (a separate
 * fetch -- see `ChannelListDTO`), already mapped to the domain shape.
 * `readme` comes from `GET /v0/arrow/:ns/readme` (a separate fetch -- see
 * `ArrowReadmeDTO`), `null` when that call 404s; `dependencies`/`dependents`
 * come from the two dependency-graph endpoints (quiver.core #220), empty when
 * nothing 200s back.
 */
export function toArrowDetail(
	detail: ArrowDetailDTO,
	manifest: ArrowManifestDTO,
	channels: ArrowChannel[],
	readme: string | null,
	dependencies: ArrowDependencyDTO[],
	dependents: string[]
): ArrowDetail {
	const meta = manifest.manifest?.metadata ?? {};
	const available = toAvailable(detail.available);
	return {
		namespace: detail.namespace,
		name: detail.name,
		description: detail.description,
		license: detail.license ?? meta.license ?? '',
		url: meta.url ?? '',
		tags: detail.tags ?? [],
		media: { icon: meta.media?.icon ?? null, banner: meta.media?.banner ?? null },
		maintainers: (meta.maintainers ?? []).map(toCredit),
		credits: (meta.credits ?? []).map(toCredit),
		netbridge: manifest.manifest?.netbridge ?? [],
		variables: manifest.variables ?? [],
		targets: toTargets(manifest.targets ?? {}),
		state: detail.state,
		user_installed: detail.user_installed,
		installed_at: detail.installed_at,
		selector: selectorOf(detail.namespace),
		selector_kind: parseSelectorKind(detail.selector_kind),
		resolved_ref: detail.resolved_ref ?? '',
		installed_commit: detail.installed_commit ?? '',
		available,
		outdated: detail.outdated ?? available !== null,
		active_run: detail.active_run ?? null,
		last_return: detail.last_return ?? null,
		origin: parseArrowOrigin(detail.origin),
		confidence: parseInferenceConfidence(detail.inference?.confidence),
		warnings: detail.inference?.warnings ?? [],
		channels,
		readme,
		dependencies: toArrowDependencies(dependencies),
		dependents: dependents ?? [],
	};
}
