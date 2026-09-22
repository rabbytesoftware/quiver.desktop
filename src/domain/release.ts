/**
 * Quiver's own arrow namespace, refless.
 *
 * The one namespace whose release assets this app can resolve, and the one it
 * announces itself under on every daemon connection.
 */
export const QUIVER_DESKTOP_NAMESPACE = 'github.com/rabbytesoftware/quiver.desktop';

/**
 * quiver.core's own arrow namespace, refless -- the daemon self-registers
 * under this exactly like any other catalogued arrow, which is what lets its
 * own self-update channel be read via the ordinary `GET /v0/arrow/:ns/channels`.
 */
export const QUIVER_CORE_NAMESPACE = 'github.com/rabbytesoftware/quiver.core';

/**
 * This app's own release asset, as `src-tauri/src/release/mod.rs` resolves it
 * from the GitHub releases API.
 *
 * Quiver's own `ARROW.md` declares `QUIVER_RELEASE_ASSET_URL` and
 * `QUIVER_RELEASE_CHECKSUM` with no defaults, which quiver.core reads as
 * "required from the caller, on every execution that reads them". Nothing in
 * the manifest can fill them in: release filenames carry `tauri.conf.json`'s
 * static `"0.1.0"` rather than the git tag, and `${REF}` during an update is
 * the ref being updated FROM. So the caller resolves the asset, and when the
 * caller is this app, that is what this type carries.
 */
export interface ResolvedReleaseAsset {
	/** The release tag the asset came from. */
	tag: string;
	/** The asset's filename. */
	name: string;
	/** `QUIVER_RELEASE_ASSET_URL`. */
	url: string;
	/**
	 * `QUIVER_RELEASE_CHECKSUM`: bare lowercase hex, no algorithm prefix.
	 *
	 * `null` when the release publishes neither a per-asset digest nor a
	 * checksum manifest. That is a fact about the release, not a verdict --
	 * see `releaseVariables` for what the app does about it.
	 */
	checksum: string | null;
}

/**
 * Why resolving failed, as the Rust side classifies it. The frontend branches
 * on `kind` to pick a sentence; `detail` is wire text, shown only where wire
 * text already belongs.
 */
export interface ReleaseResolveError {
	kind: ReleaseResolveErrorKind;
	detail: string;
}

export type ReleaseResolveErrorKind =
	/** DNS, TLS, connect or timeout -- the machine could not reach GitHub. */
	| 'offline'
	/** GitHub refused on quota. Unauthenticated callers share 60 requests an hour per IP. */
	| 'rate_limited'
	/** The repository publishes no release the API will name. */
	| 'no_release'
	/** Any other non-success status. */
	| 'http'
	/** A 2xx whose body was not a release document. */
	| 'malformed'
	/** The release carries nothing installable on this OS and architecture. */
	| 'no_asset'
	/** This build is running somewhere no Quiver bundle is published for. */
	| 'unsupported_platform'
	/**
	 * Not produced by Rust: the asset resolved, but nothing published a
	 * checksum for it. Raised on this side, because whether that blocks an
	 * install is a policy decision rather than a resolution failure.
	 */
	| 'unverifiable';

/** Whether an unknown value from the IPC boundary is one of these errors. */
export function isReleaseResolveError(value: unknown): value is ReleaseResolveError {
	return (
		typeof value === 'object' &&
		value !== null &&
		typeof (value as ReleaseResolveError).kind === 'string' &&
		typeof (value as ReleaseResolveError).detail === 'string'
	);
}
