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
