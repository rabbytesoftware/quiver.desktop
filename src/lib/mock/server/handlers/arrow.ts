import { findArrow } from '../../world/types';
import { fail, ok } from '../envelope';
import {
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
			// core rejects `namespace@ref` here the same way it already does for `/manifest` (`ErrInvalidNamespace`, 400).
			if (req.params.ns.includes('@')) return fail('invalid namespace', 400);

			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			if (!arrow.readme) return fail(`arrow ${req.params.ns} has no readme`, 404);
			return ok(toArrowReadmeDTO(req.params.ns, arrow.readme));
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
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			if (arrow.user_installed) return fail(`arrow ${req.params.ns} is already in the library`, 500);

			arrow.user_installed = true;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'upserted'));
			return ok(null);
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
