/**
 * Wire types for the two search lanes.
 *
 * Both lanes carry the same result shape: core builds one model and hands it to
 * one mapper, so `GET /v0/search` and the discovery stream cannot drift into two
 * shapes the client would have to render differently.
 */

/**
 * How this machine came to hold an arrow. Absent when the server cannot say --
 * a streamed result the catalog already holds is known to be yours, but
 * discovery does not know which provenance the catalog recorded for it.
 */
export type SearchProvenance = 'installed' | 'dependency' | 'collection' | 'seen';

export interface SearchResultDTO {
	/** Bare, without a ref. `versions` carries the refs. */
	namespace: string;
	name: string;
	description: string;
	tags: string[];
	media?: {
		icon?: string | null;
		banner?: string | null;
	};
	/** Refs known locally, not every ref that exists upstream. */
	versions: string[];
	/**
	 * Advisory. A denormalised projection of the last compile; install-time
	 * re-resolution is authoritative.
	 */
	compatible_os: string[];
	provenance?: SearchProvenance;
	/**
	 * The catalog holds this arrow -- the only thing separating what you have
	 * from what you could have. Narrower than `known` and never implied by it:
	 * an arrow can be on this machine because a search cached its manifest,
	 * which installs nothing.
	 */
	installed: boolean;
	/**
	 * On this machine by either route: the catalog holds it, or the vault index
	 * cached its manifest. Every `GET /v0/search` result is known by
	 * construction. When merging a streamed result over a row already rendered,
	 * keep the existing row if this is true -- otherwise a correct provenance is
	 * overwritten with a less specific one.
	 */
	known: boolean;
	stars: number;
	source?: string;
}

/**
 * The answer to `POST /v0/search/discover`, returned before any provider has
 * been asked. It carries no counts: those live on the job and are read once,
 * after the stream closes.
 */
export interface DiscoveryJobStartedDTO {
	job_id: string;
	query: string;
	/**
	 * When the job stops being readable. While the pass runs this is a floor
	 * rather than a promise -- the grace is measured from when the pass ends.
	 */
	expires_at: string;
}

/** What one host contributed. A host that was never asked has no entry. */
export interface DiscoveryProviderDTO {
	host: string;
	ok: boolean;
	returned: number;
	/** Why the host refused, so a refusal never renders as "no results". */
	reason?: string;
	/** Seconds. Set only when the host said how long to wait. */
	retry_after?: number;
}

export type DiscoveryJobStatus = 'running' | 'completed';

/**
 * Everything the stream cannot carry. A stream is one payload type, so the
 * counts and the hosts that refused are reported here rather than as extra
 * frame shapes. Read once, after the socket closes: the counts and providers
 * are written when the pass ends, so a job read mid-pass reports zeroes and an
 * empty provider list.
 */
export interface DiscoveryJobDTO {
	job_id: string;
	status: DiscoveryJobStatus;
	query: string;
	found: number;
	verified: number;
	skipped: number;
	providers: DiscoveryProviderDTO[];
}
