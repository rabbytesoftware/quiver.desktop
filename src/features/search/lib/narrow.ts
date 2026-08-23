import type { SearchEntry } from '@/domain/search';

/**
 * The two things a result set can be narrowed by.
 *
 * `compatible_os` is deliberately not among them. Spec 1.1 rules it out on its
 * own grounds, separate from the argument the caption retires: the list is a
 * projection of the last compile and install-time re-resolution is
 * authoritative, so filtering on it would hide arrows that would in fact
 * install. It is shown on the card and never offered as a control.
 */
export type FacetKind = 'host' | 'tag';

export interface Facet {
	value: string;
	count: number;
}

export type Selection = Readonly<Record<FacetKind, readonly string[]>>;

export const NO_SELECTION: Selection = { host: [], tag: [] };

export function isNarrowed(selection: Selection): boolean {
	return selection.host.length > 0 || selection.tag.length > 0;
}

/** `github.com/PaperMC/Paper` -> `github.com`. */
export function hostOf(entry: SearchEntry): string {
	return entry.namespace.split('/')[0];
}

function valuesOf(entry: SearchEntry, kind: FacetKind): string[] {
	return kind === 'host' ? [hostOf(entry)] : entry.tags;
}

/**
 * Counted off the results themselves, so a facet can never name something the
 * list does not contain.
 *
 * A value carried by all but one result is dropped: it is the query restated,
 * and selecting it would remove a single card while looking like a filter.
 */
export function facetsFor(entries: SearchEntry[], kind: FacetKind, take: number): Facet[] {
	const counts = new Map<string, number>();
	for (const entry of entries) {
		for (const value of valuesOf(entry, kind)) {
			counts.set(value, (counts.get(value) ?? 0) + 1);
		}
	}

	return [...counts.entries()]
		.filter(([, count]) => kind === 'host' || count <= entries.length - 2)
		.sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
		.slice(0, take)
		.map(([value, count]) => ({ value, count }));
}

/** Selecting inside one facet widens (OR); selecting across facets narrows (AND). */
export function applySelection(entries: SearchEntry[], selection: Selection): SearchEntry[] {
	const chosen = { host: new Set(selection.host), tag: new Set(selection.tag) };
	return entries.filter((entry) =>
		(['host', 'tag'] as const).every((kind) => {
			const wanted = chosen[kind];
			if (wanted.size === 0) return true;
			return valuesOf(entry, kind).some((value) => wanted.has(value));
		})
	);
}

export function toggle(selection: Selection, kind: FacetKind, value: string): Selection {
	const chosen = selection[kind];
	const next = chosen.includes(value) ? chosen.filter((each) => each !== value) : [...chosen, value];
	return { ...selection, [kind]: next };
}
