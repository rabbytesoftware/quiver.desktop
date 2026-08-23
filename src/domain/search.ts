/** Null when the server cannot say, not when there is none. */
export type SearchProvenance = 'installed' | 'dependency' | 'collection' | 'seen';

export interface SearchEntry {
	/** Bare, without a ref, unlike `ArrowEntry.namespace`. `versions` carries the refs. */
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	icon: string | null;
	banner: string | null;
	versions: string[];
	compatible_os: string[];
	provenance: SearchProvenance | null;
	/** The catalog holds it. Narrower than `known` and never implied by it. */
	installed: boolean;
	/** The catalog or the vault holds it. Browsing is not having. */
	known: boolean;
	stars: number;
	source: string | null;
}

export interface DiscoveryProvider {
	host: string;
	ok: boolean;
	returned: number;
	reason: string | null;
	/** Seconds. */
	retry_after: number | null;
}

export interface DiscoverySummary {
	job_id: string;
	query: string;
	found: number;
	verified: number;
	skipped: number;
	providers: DiscoveryProvider[];
}

/**
 * Whether the arrow is already yours, which is what separates the two result
 * shelves (spec 9.3).
 *
 * `installed` carries this: core sets it from the catalog alone, and the DTO
 * calls it "the only thing that separates what you have from what you could
 * have". `known` cannot -- every `GET /v0/search` result is known by
 * construction, because discovery indexes each arrow it proves and the settle
 * re-query reads that index back.
 *
 * Following a collection is the exception the boolean misses: it caches the
 * collection's arrows into the vault without writing catalog rows, so a curated
 * arrow reports `installed: false` while plainly being yours.
 */
export function isHeld(entry: SearchEntry): boolean {
	return entry.installed || entry.provenance === 'collection';
}
