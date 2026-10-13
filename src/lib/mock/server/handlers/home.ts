import type { MockWorld } from '../../world/types';
import { accepted, ok } from '../envelope';
import { toSearchResultDTO } from '../projections';
import type { Route } from '../router';

const SHELF_SIZE = 6;

function inCatalog(world: MockWorld, namespace: string): boolean {
	for (const arrow of world.arrows.values()) if (arrow.namespace === namespace) return true;
	return false;
}

/** One snapshot shelf over what a pass could find; empty worlds get no shelf, like a disabled core. */
export const homeRoutes: Route[] = [
	{
		method: 'GET',
		pattern: '/v0/home',
		fault: 'home',
		handler: (_req, world) => {
			const arrows = world.discoverable.slice(0, SHELF_SIZE).map(({ arrow }) =>
				toSearchResultDTO(arrow, {
					refs: [arrow.ref],
					installed: inCatalog(world, arrow.namespace),
					known: world.vault.has(arrow.namespace),
				})
			);
			if (arrows.length === 0) return ok({ shelves: [], refreshing: false });
			return ok({
				shelves: [{ id: 'popular', title: 'Popular', refreshed_at: new Date().toISOString(), arrows }],
				refreshing: false,
			});
		},
	},
	{
		method: 'POST',
		pattern: '/v0/home/refresh',
		fault: 'home',
		handler: () => accepted(),
	},
];
