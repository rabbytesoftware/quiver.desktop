import type {
	ActiveRun,
	ArrowCredit,
	ArrowDependency,
	ArrowDetail,
	ArrowLifecycle,
	ArrowMethod,
	ArrowState,
	ArrowStepDefinition,
	ArrowTarget,
	DependencyType,
	InstalledVersion,
	RuntimeUpdate,
	StepProgress,
} from '@/domain/arrow';
import { splitNamespace } from '@/lib/namespace';
import type { ArrowCatalogRecord } from '@/lib/persistence/schemas';

export interface InstalledVersionDTO {
	ref: string;
	version: string;
	state: ArrowState;
	installed_at?: string;
	/** Set once quiver.core stamps it on a completed `execute` (enhancement/last_used); absent until then. */
	last_used_at?: string;
}

export interface ArrowListResponseItemDTO {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	media?: {
		icon?: string | null;
		banner?: string | null;
	};
	versions: InstalledVersionDTO[];
}

export interface LastReturnDTO {
	method: string;
	outcome: 'success' | 'failed' | 'cancelled';
	variables: Record<string, string>;
	steps: StepProgress[];
}

export interface ArrowDetailDTO {
	namespace: string;
	name: string;
	version: string;
	description: string;
	license: string;
	state: ArrowState;
	tags: string[];
	/**
	 * Absent, not just empty, when `namespace` already carries the resolved ref
	 * itself -- quiver.core PR #225 made `GetDetail` resolve live for an
	 * uncatalogued namespace, and the live-resolved case stamps the ref onto
	 * `namespace` rather than reporting it here.
	 */
	installed_ref?: string;
	installed_at: string;
	installed_constraint?: string;
	user_installed: boolean;
	active_run?: ActiveRun | null;
	last_return?: LastReturnDTO | null;
}

export function toArrowCatalogRecords(items: ArrowListResponseItemDTO[], connectionId: string): ArrowCatalogRecord[] {
	return items.flatMap((arrow) =>
		arrow.versions.map((v) => ({
			connectionId,
			namespace: `${arrow.namespace}@${v.ref}`,
			name: arrow.name,
			description: arrow.description,
			tags: arrow.tags,
			icon: arrow.media?.icon || null,
			banner: arrow.media?.banner || null,
			version: v.version,
			last_used_at: v.last_used_at ?? null,
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

/** The raw manifest, nested whole under `manifest` -- the only place url/maintainers/credits/media/netbridge live on the wire. */
export interface RawManifestDTO {
	url: string;
	maintainers: CreditDTO[];
	credits: CreditDTO[];
	media: ArrowMediaDTO;
	netbridge: PortDTO[];
}

export interface ArrowManifestDTO {
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	variables: VariableDTO[];
	targets: Record<string, TargetManifestDTO>;
	manifest: RawManifestDTO;
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

/**
 * Merges the five real endpoints into the one shape the Arrow Details page
 * consumes. `versions` comes from the catalog, not any of the five; `readme`
 * comes from `GET /v0/arrow/:ns/readme` (a separate fetch -- see `ArrowReadmeDTO`),
 * `null` when that call 404s; `dependencies`/`dependents` come from the two
 * dependency-graph endpoints (quiver.core #220), empty when nothing 200s back.
 */
export function toArrowDetail(
	detail: ArrowDetailDTO,
	manifest: ArrowManifestDTO,
	versions: InstalledVersion[],
	readme: string | null,
	dependencies: ArrowDependencyDTO[],
	dependents: string[]
): ArrowDetail {
	// A catalogued arrow's `namespace` comes back bare, with the ref reported
	// separately as `installed_ref`. A live-resolved-but-uncatalogued one
	// (quiver.core PR #225) already carries its resolved ref on `namespace`
	// itself and omits `installed_ref` -- use it as-is rather than appending
	// a second, absent ref.
	const { head, tail } = splitNamespace(detail.namespace);
	const namespace = tail ? detail.namespace : `${head}@${detail.installed_ref}`;
	// Non-null: `installed_ref` is only ever absent on the wire when `tail`
	// already supplied the ref instead (see `ArrowDetailDTO.installed_ref`).
	const installedRef = tail ? tail.slice(1) : detail.installed_ref!;
	return {
		// Every runtime/arrow endpoint this app calls afterwards
		// (install/execute/stop/etc, and re-fetching this same detail) expects
		// the full `namespace@ref` form, matching the route param and the
		// reactive store's own keying (`versioned()`). Resolving it here means
		// every downstream consumer (Hero, the action mutations, ActionButton)
		// can just use `detail.namespace` as-is.
		namespace,
		name: detail.name,
		description: detail.description,
		license: detail.license,
		url: manifest.manifest.url,
		tags: detail.tags,
		media: { icon: manifest.manifest.media.icon ?? null, banner: manifest.manifest.media.banner ?? null },
		maintainers: manifest.manifest.maintainers.map(toCredit),
		credits: manifest.manifest.credits.map(toCredit),
		netbridge: manifest.manifest.netbridge,
		variables: manifest.variables,
		targets: toTargets(manifest.targets),
		state: detail.state,
		user_installed: detail.user_installed,
		installed_ref: installedRef,
		installed_at: detail.installed_at,
		installed_constraint: detail.installed_constraint,
		active_run: detail.active_run ?? null,
		last_return: detail.last_return ?? null,
		versions,
		readme,
		dependencies: dependencies.map(toDependency),
		dependents,
	};
}
