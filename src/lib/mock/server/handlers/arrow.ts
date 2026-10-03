import { selectorOf } from '@/lib/namespace';

import {
	classifySelector,
	findArrow,
	findRepository,
	resolvedRefOf,
	versioned,
	type MockArrow,
	type MockWorld,
} from '../../world/types';
import { accepted, fail, mutated, ok } from '../envelope';
import {
	toArrowChannelsDTO,
	toArrowDependenciesDTO,
	toArrowDependentsDTO,
	toArrowDetailDTO,
	toArrowFrame,
	toArrowListDTO,
	toArrowManifestDTO,
	toArrowReadmeDTO,
} from '../projections';
import type { Route } from '../router';

const ARROW_ENDPOINT = '/v0/arrow';

/**
 * A registration of a selector the world has no row for: a new row of the same
 * repository, filed under that selector, nothing on disk yet -- what core's
 * register does. Returns undefined for a repository nothing in the world is.
 */
function registerNewRow(world: MockWorld, ns: string): MockArrow | undefined {
	const repository = findRepository(world.arrows, ns);
	if (!repository) return undefined;
	const selector = selectorOf(ns);
	const row: MockArrow = {
		...repository,
		ref: selector,
		selector_kind: classifySelector(selector, repository.channels),
		resolved_ref: undefined,
		available: undefined,
		user_installed: true,
		state: 'absent',
		installed_at: '',
		active_run: null,
		last_return: null,
	};
	row.resolved_ref = resolvedRefOf(row);
	world.arrows.set(versioned(row), row);
	return row;
}

export const arrowRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/arrow',
		fault: 'arrows',
		handler: (req, world) => {
			const onlyLibrary = req.query.get('user_installed') === 'true';
			const arrows = [...world.arrows.values()].filter((a) => !onlyLibrary || a.user_installed);
			return ok(toArrowListDTO(arrows));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowDetailDTO(arrow));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns/manifest',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowManifestDTO(arrow));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns/readme',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			if (!arrow.readme) return fail(`arrow ${req.params.ns} has no readme`, 404);
			return ok(toArrowReadmeDTO(req.params.ns, arrow.readme));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns/channels',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowChannelsDTO(arrow));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns/dependencies',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowDependenciesDTO(arrow));
		},
	},
	{
		method: 'GET',
		pattern: '/v0/arrow/:ns/dependents',
		fault: 'arrow-detail',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowDependentsDTO(arrow, [...world.arrows.values()]));
		},
	},
	{
		method: 'POST',
		pattern: '/v0/arrow/:ns',
		fault: 'arrows',
		handler: (req, world) => {
			const existing = findArrow(world.arrows, req.params.ns);
			const arrow = existing ?? registerNewRow(world, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			// Re-registering an identity already in the library is a no-op, the
			// way core answers it -- 201 all the same.
			arrow.user_installed = true;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'upserted'));
			return mutated(versioned(arrow));
		},
	},
	{
		// Declares `resolved_ref` as what is already installed under the
		// identity: a new row when there is none, an in-place advance otherwise.
		// Core also checks the ref against the repository's refs; the mock has
		// no ref snapshot to check it against, so it takes the ref as given.
		method: 'POST',
		pattern: '/v0/arrow/:ns/adopt',
		fault: 'arrows',
		handler: (req, world) => {
			const body = (req.body ?? {}) as { resolved_ref?: string };
			if (!body.resolved_ref) return fail('resolved_ref is required', 400);
			const arrow = findArrow(world.arrows, req.params.ns) ?? registerNewRow(world, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			arrow.user_installed = true;
			arrow.resolved_ref = body.resolved_ref;
			arrow.available = undefined;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'upserted'));
			return mutated(versioned(arrow));
		},
	},
	{
		// A re-check, no body. A row with nothing installed advances in place at
		// once; an installed one stays put and reports what is available, which
		// only `POST /v0/runtime/:ns/update` moves it to.
		method: 'PATCH',
		pattern: '/v0/arrow/:ns',
		fault: 'arrows',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			if (arrow.available && arrow.state === 'absent') {
				arrow.resolved_ref = arrow.available.ref;
				arrow.available = undefined;
				world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'upserted'));
			}
			return ok({
				added_deps: [],
				removed_from_manifest: [],
				safe_to_uninstall: [],
				constrained_deps: [],
				...(arrow.available ? { available: arrow.available } : {}),
			});
		},
	},
	{
		// Answers before the check has run, like core: whatever it finds reaches
		// the app as a runtime frame. The mock has nothing to find.
		method: 'POST',
		pattern: '/v0/arrow/:ns/check',
		fault: 'arrows',
		handler: (req, world) => {
			if (!findArrow(world.arrows, req.params.ns)) return fail(`arrow ${req.params.ns} not found`, 404);
			return accepted();
		},
	},
	{
		method: 'DELETE',
		pattern: '/v0/arrow/:ns',
		fault: 'arrows',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			arrow.user_installed = false;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'removed'));
			return ok(null);
		},
	},
];
