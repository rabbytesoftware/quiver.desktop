import { fail, ok } from '../envelope';
import { toCollectionDetailDTO, toCollectionListDTO } from '../projections';
import type { Route } from '../router';

export const collectionRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/collection',
		fault: 'collections',
		handler: (_req, world) => ok([...world.collections.values()].map(toCollectionListDTO)),
	},
	{
		method: 'GET',
		pattern: '/v0/collection/:ns',
		fault: 'collection-detail',
		handler: (req, world) => {
			const collection = world.collections.get(req.params.ns);
			if (!collection) return fail(`collection ${req.params.ns} not found`, 404);
			// Members carry their own `resolved` flag rather than being filtered
			// out. A collection naming an arrow the daemon cannot see is not a
			// broken collection, and silently dropping the row would turn a
			// four-arrow pack into a three-arrow one with no explanation.
			return ok(toCollectionDetailDTO(collection));
		},
	},
	{
		method: 'POST',
		pattern: '/v0/collection/:ns/follow',
		fault: 'collections',
		handler: (req, world) => {
			const collection = world.collections.get(req.params.ns);
			if (!collection) return fail(`collection ${req.params.ns} not found`, 404);
			collection.followed = true;
			return ok(null);
		},
	},
	{
		method: 'DELETE',
		pattern: '/v0/collection/:ns/follow',
		fault: 'collections',
		handler: (req, world) => {
			const collection = world.collections.get(req.params.ns);
			if (!collection) return fail(`collection ${req.params.ns} not found`, 404);
			collection.followed = false;
			return ok(null);
		},
	},
];
