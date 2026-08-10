import { fail, ok } from '../envelope';
import { toArrowDetailDTO, toArrowFrame, toArrowListDTO } from '../projections';
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
			const arrow = world.arrows.get(req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			return ok(toArrowDetailDTO(arrow));
		},
	},
	{
		method: 'POST',
		pattern: '/v0/arrow/:ns',
		fault: 'arrows',
		handler: (req, world) => {
			const arrow = world.arrows.get(req.params.ns);
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
			const arrow = world.arrows.get(req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			arrow.user_installed = false;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'removed'));
			return ok(null);
		},
	},
];
