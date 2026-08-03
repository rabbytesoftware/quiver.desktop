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
			// The app always asks with `user_installed=true`; honouring the flag
			// rather than ignoring it is what keeps the library and the searchable
			// universe two different sets, which is the only way search can return
			// something the rail does not already show.
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
			// Core 500s on a POST to a namespace already in the library. Matched
			// here because it is the reason no live `upserted` frame has ever been
			// captured from a real daemon — worth the app continuing to meet it.
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
			// A tombstone, not a delete: the arrow stays in the world so search can
			// still find it — leaving the library is not ceasing to exist.
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'removed'));
			return ok(null);
		},
	},
];
