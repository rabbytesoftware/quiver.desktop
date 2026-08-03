import { providersFor } from '../../world/build';
import type { MockArrow } from '../../world/types';
import { versioned } from '../../world/types';
import { fail, ok } from '../envelope';
import { toDiscoveryJobDTO, toSearchResultDTO } from '../projections';
import type { Route } from '../router';

const DISCOVER_MS = 1500;

function matches(arrow: MockArrow, q: string): boolean {
	const needle = q.toLowerCase();
	return (
		arrow.name.toLowerCase().includes(needle) ||
		arrow.description.toLowerCase().includes(needle) ||
		arrow.namespace.toLowerCase().includes(needle) ||
		arrow.tags.some((tag) => tag.toLowerCase().includes(needle))
	);
}

export const searchRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/search',
		fault: 'search',
		handler: (req, world) => {
			const q = req.query.get('q') ?? '';
			// Empty query returns the whole shelf: the screen opens before you type.
			const hits = [...world.arrows.values()].filter((a) => q === '' || matches(a, q));
			return ok(hits.map(toSearchResultDTO));
		},
	},
	{
		method: 'POST',
		pattern: '/v0/search/discover',
		fault: 'discover',
		handler: (req, world) => {
			const body = (req.body ?? {}) as { q?: string; query?: string };
			const query = body.q ?? body.query ?? '';
			const id = `job-${world.nextId()}`;

			// Born running with no results, so the pass is observable.
			world.jobs.set(id, { id, status: 'running', query, providers: [], results: [] });

			world.clock.after(DISCOVER_MS, () => {
				const job = world.jobs.get(id);
				if (!job) return;
				const providers = providersFor(world.scenario);
				const hits = [...world.arrows.values()].filter((a) => query === '' || matches(a, query));
				// Only hosts that answered contribute results.
				const returnable = providers.filter((p) => p.ok).reduce((sum, p) => sum + p.returned, 0);
				job.providers = providers;
				job.results = hits.slice(0, returnable).map((a) => versioned(a));
				job.status = 'done';
			});

			// 200, NOT 202, even though this starts async work — `apiFetch` reads a
			// 202 as "success with no payload" and discards the body, and the
			// caller needs the job id to poll for. The status code and the
			// semantics disagree here; the client's contract wins.
			return ok(toDiscoveryJobDTO(world.jobs.get(id)!));
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
