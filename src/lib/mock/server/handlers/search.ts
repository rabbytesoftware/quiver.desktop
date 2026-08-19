import type { SearchProvenance } from '@/domain/search';

import { providersFor } from '../../world/build';
import type { MockArrow, MockCandidate, MockWorld } from '../../world/types';
import { fail, ok } from '../envelope';
import { toDiscoveryJobDTO, toDiscoveryStartedDTO, toSearchResultDTO } from '../projections';
import type { Route } from '../router';

const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 100;

/** How long a finished job stays readable, matching core's grace. */
const JOB_GRACE_MS = 30_000;

/**
 * Results are staggered rather than delivered together. A pass that lands in
 * one burst cannot show the behaviour the screen is built around -- rows
 * arriving over time, into a space reserved before the first one exists.
 */
const FIRST_RESULT_MS = 220;
const BETWEEN_RESULTS_MS = 260;
/** Slack after the last result, so the summary is not written mid-stream. */
const CLOSE_MS = 300;

function matches(arrow: MockArrow, q: string): boolean {
	const needle = q.toLowerCase();
	return (
		arrow.name.toLowerCase().includes(needle) ||
		arrow.description.toLowerCase().includes(needle) ||
		arrow.namespace.toLowerCase().includes(needle) ||
		arrow.tags.some((tag) => tag.toLowerCase().includes(needle))
	);
}

function parseLimit(raw: string | null): number {
	const n = Number.parseInt(raw ?? '', 10);
	if (!Number.isFinite(n) || n < 1) return DEFAULT_LIMIT;
	return Math.min(n, MAX_LIMIT);
}

/**
 * Why the catalog holds an arrow. A followed collection wins over the install
 * flag: it is the more specific answer, and it is the one a merging client must
 * not lose.
 */
function provenanceOf(world: MockWorld, namespace: string): SearchProvenance {
	for (const collection of world.collections.values()) {
		if (!collection.followed) continue;
		if (collection.arrows.some((m) => m.namespace.split('@')[0] === namespace)) return 'collection';
	}
	const anyUserInstalled = [...world.arrows.values()].some((a) => a.namespace === namespace && a.user_installed);
	return anyUserInstalled ? 'installed' : 'dependency';
}

interface CatalogRow {
	arrow: MockArrow;
	refs: string[];
	installed: boolean;
	known: boolean;
	provenance: SearchProvenance;
}

/**
 * Groups the catalog by bare namespace, the way core reports one row per
 * arrow, then unions in vault-only namespaces (spec 1.1: Lane A reads the
 * catalog *and* the vault index). A vault row is `installed: false, known:
 * true` with provenance `'seen'` -- it is on the machine because a pass
 * cached its manifest, which installs nothing. Without this union the §3.5
 * settle re-query sees the same rows it started with and every streamed
 * result vanishes.
 */
function catalogRows(world: MockWorld, q: string): CatalogRow[] {
	const byNamespace = new Map<string, MockArrow[]>();
	for (const arrow of world.arrows.values()) {
		if (!matches(arrow, q)) continue;
		const group = byNamespace.get(arrow.namespace) ?? [];
		group.push(arrow);
		byNamespace.set(arrow.namespace, group);
	}
	const catalog: CatalogRow[] = [...byNamespace.values()].map((group) => ({
		arrow: group[0],
		refs: group.map((a) => a.ref),
		installed: true,
		known: true,
		provenance: provenanceOf(world, group[0].namespace),
	}));

	const vaultRows: CatalogRow[] = [];
	for (const namespace of world.vault) {
		if (byNamespace.has(namespace)) continue;
		const candidate = world.discoverable.find((c) => c.arrow.namespace === namespace);
		if (!candidate || !matches(candidate.arrow, q)) continue;
		vaultRows.push({
			arrow: candidate.arrow,
			refs: [candidate.arrow.ref],
			installed: false,
			known: true,
			provenance: 'seen',
		});
	}

	return [...catalog, ...vaultRows];
}

function hasInCatalog(world: MockWorld, namespace: string): boolean {
	for (const arrow of world.arrows.values()) if (arrow.namespace === namespace) return true;
	return false;
}

/**
 * Runs the pass. Each candidate is indexed into the vault before it is emitted,
 * exactly as core does, so `known` is read against the state that existed when
 * the pass looked -- and an arrow found by an earlier pass comes back known
 * without ever becoming installed.
 */
function runPass(world: MockWorld, jobId: string, hits: MockCandidate[]): void {
	const endpoint = `/v0/search/discover/${jobId}`;
	const verifiable = hits.filter((c) => c.verifiable);

	verifiable.forEach((candidate, i) => {
		world.clock.after(FIRST_RESULT_MS + i * BETWEEN_RESULTS_MS, () => {
			const job = world.jobs.get(jobId);
			if (!job || job.status === 'completed') return;

			const namespace = candidate.arrow.namespace;
			const inCatalog = hasInCatalog(world, namespace);
			const inVault = world.vault.has(namespace);

			world.vault.add(namespace);

			world.emitter.emit(
				endpoint,
				toSearchResultDTO(candidate.arrow, {
					refs: [candidate.arrow.ref],
					installed: inCatalog,
					known: inCatalog || inVault,
					provenance: inCatalog ? undefined : 'seen',
				})
			);
			job.verified += 1;
		});
	});

	const closeAt = FIRST_RESULT_MS + verifiable.length * BETWEEN_RESULTS_MS + CLOSE_MS;
	world.clock.after(closeAt, () => {
		const job = world.jobs.get(jobId);
		if (!job) return;

		const returnedByHost = new Map<string, number>();
		for (const candidate of hits) {
			const host = candidate.arrow.source ?? 'github.com';
			returnedByHost.set(host, (returnedByHost.get(host) ?? 0) + 1);
		}

		job.providers = providersFor(world.scenario).map((p) => ({
			...p,
			returned: p.ok ? (returnedByHost.get(p.host) ?? 0) : 0,
		}));
		job.found = hits.length;
		job.skipped = hits.length - job.verified;
		job.status = 'completed';
		job.expires_at = new Date(Date.now() + JOB_GRACE_MS).toISOString();
	});
}

export const searchRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/search',
		fault: 'search',
		handler: (req, world) => {
			const q = (req.query.get('q') ?? '').trim();
			if (q === '') return fail('query parameter q is required', 400);

			const rows = catalogRows(world, q).slice(0, parseLimit(req.query.get('limit')));

			return ok(
				rows.map(({ arrow, refs, installed, known, provenance }) =>
					toSearchResultDTO(arrow, { refs, installed, known, provenance })
				)
			);
		},
	},
	{
		method: 'POST',
		pattern: '/v0/search/discover',
		fault: 'discover',
		handler: (req, world) => {
			const body = (req.body ?? {}) as { q?: string; query?: string };
			const query = (body.q ?? body.query ?? '').trim();
			if (query === '') return fail('query parameter q is required', 400);

			const id = `job-${world.nextId()}`;
			world.jobs.set(id, {
				id,
				status: 'running',
				query,
				expires_at: new Date(Date.now() + JOB_GRACE_MS).toISOString(),
				found: 0,
				verified: 0,
				skipped: 0,
				providers: [],
			});

			runPass(
				world,
				id,
				world.discoverable.filter((c) => matches(c.arrow, query))
			);

			// 202 with the ticket, not the job: nothing has been asked yet.
			return ok(toDiscoveryStartedDTO(world.jobs.get(id)!), 202);
		},
	},
	{
		method: 'GET',
		pattern: '/v0/search/discover/:job',
		fault: 'discover',
		handler: (req, world) => {
			const job = world.jobs.get(req.params.job);
			if (!job) return fail(`discovery job ${req.params.job} not found`, 404);
			return ok(toDiscoveryJobDTO(job));
		},
	},
];
