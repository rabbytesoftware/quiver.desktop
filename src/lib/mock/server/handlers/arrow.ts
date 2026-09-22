import { findArrow, versioned } from '../../world/types';
import { fail, ok } from '../envelope';
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
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);
			if (arrow.user_installed) return fail(`arrow ${req.params.ns} is already in the library`, 500);

			const body = (req.body ?? {}) as { channel?: string };
			arrow.user_installed = true;
			if (body.channel) arrow.channel = body.channel;
			world.emitter.emit(ARROW_ENDPOINT, toArrowFrame(arrow, 'upserted'));
			return ok(null);
		},
	},
	{
		method: 'PATCH',
		pattern: '/v0/arrow/:ns',
		fault: 'arrows',
		handler: (req, world) => {
			const arrow = findArrow(world.arrows, req.params.ns);
			if (!arrow) return fail(`arrow ${req.params.ns} not found`, 404);

			const body = (req.body ?? {}) as { channel?: string; ref?: string };
			if (!body.channel) return fail('channel is required', 400);

			// Real quiver.core resolves an omitted `ref` to the channel's own
			// `latest` -- mirror that here rather than leaving the ref untouched.
			const entry = (arrow.channels ?? []).find((c) => c.name === body.channel);
			const nextRef = body.ref ?? entry?.latest ?? arrow.ref;

			// `world.arrows` is keyed by `namespace@ref` (see `findArrow`'s own
			// comment) -- changing `ref` in place without re-keying would strand
			// this entry under its old key, so any future exact-key lookup for
			// the new ref would miss it.
			const oldKey = versioned(arrow);
			arrow.channel = body.channel;
			arrow.ref = nextRef;
			if (versioned(arrow) !== oldKey) {
				world.arrows.delete(oldKey);
				world.arrows.set(versioned(arrow), arrow);
			}

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
