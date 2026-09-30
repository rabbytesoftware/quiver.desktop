import type { Home, HomeShelf } from '@/domain/home';
import { isHeld } from '@/domain/search';

/** The shelves with held arrows dropped, and any shelf left empty removed. */
export function visibleShelves(home: Home | undefined): HomeShelf[] {
	return (home?.shelves ?? []).flatMap((shelf) => {
		const arrows = shelf.arrows.filter((arrow) => !isHeld(arrow));
		return arrows.length > 0 ? [{ ...shelf, arrows }] : [];
	});
}

/** True when the snapshot holds no arrow at all, held or not. */
export function isSnapshotEmpty(home: Home | undefined): boolean {
	return (home?.shelves ?? []).every((shelf) => shelf.arrows.length === 0);
}
