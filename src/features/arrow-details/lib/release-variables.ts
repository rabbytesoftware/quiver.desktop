import type { ReleaseResolveError, ReleaseResolveErrorKind } from '@/domain/release';
import { isReleaseResolveError, QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import type { MessageKey } from '@/lib/i18n';
import { backend } from '@/lib/transport/backend';

/** The two variables `ARROW.md` declares without defaults. */
export const RELEASE_ASSET_URL = 'QUIVER_RELEASE_ASSET_URL';
export const RELEASE_CHECKSUM = 'QUIVER_RELEASE_CHECKSUM';

export const RELEASE_VARIABLE_NAMES: readonly string[] = [RELEASE_ASSET_URL, RELEASE_CHECKSUM];

/**
 * Whether this arrow is Quiver itself.
 *
 * The resolver reads quiver.desktop's OWN releases, so it is only ever
 * correct for quiver.desktop's own row. Refs are stripped because the same
 * app appears under whichever ref it announced itself at, and because an
 * update is started against the row's ref (the one being updated FROM) while
 * the asset comes from the newest release.
 */
export function isSelfArrow(namespace: string): boolean {
	return namespace.split('@')[0] === QUIVER_DESKTOP_NAMESPACE;
}

/**
 * Resolves this app's own release asset into the two execution variables
 * quiver.core requires for `install` and `update`.
 *
 * WHY THIS RUNS AT CLICK TIME rather than being baked into the manifest: see
 * the module comment on `src-tauri/src/release/mod.rs`. In short, the asset
 * is not derivable from anything the manifest knows, and `${REF}` during an
 * update names the version the user is leaving, not the one they are going
 * to.
 *
 * WHY IT IS RESOLVED FRESH ON EVERY CALL, and never cached or reused: the
 * variable-resolution layer behind these two values (quiver.core's assembler,
 * layer 5) used to carry a previous execution's variables forward, so a bare
 * re-update could silently re-fetch and re-install the asset from the LAST
 * update instead of failing. That layer no longer supplies a variable the
 * manifest declared without a default, and this side independently never
 * relies on it: every click resolves again and sends both values, so the only
 * value the engine can use is the one just read off the releases API.
 *
 * THE UNVERIFIABLE CASE. `install.sh` warns and installs anyway when a
 * release publishes no checksum, on the grounds that the download still came
 * from GitHub over TLS and refusing would leave the user with no app at all.
 * This path does NOT inherit that call, for two reasons that only apply here.
 * First, it cannot: `ARROW.md` verifies `${QUIVER_RELEASE_CHECKSUM}`, and
 * quiver.core's fetch step deliberately refuses a `${...}` checksum that
 * resolved to empty rather than reading it as "skip verification"
 * (`ErrChecksumUnresolved`), so "proceed unverified" is not a state this
 * lifecycle can be put into -- offering it would mean promising a user
 * something that then fails mid-update with the app already killed. Second,
 * the cost is not the same: refusing an in-app update leaves a working app on
 * screen and the one-line installer still available, where refusing a first
 * install leaves nothing. What HAS been made consistent is the input: both
 * this and the installers now read GitHub's own per-asset `digest` as well as
 * a published checksum manifest, so both verify in every case where anything
 * is publishable at all.
 */
export async function releaseVariables(): Promise<Record<string, string>> {
	const asset = await backend()
		.resolveReleaseAsset()
		.catch((err: unknown) => {
			throw asResolveError(err);
		});

	if (!asset.checksum) {
		const failure: ReleaseResolveError = {
			kind: 'unverifiable',
			detail: `${asset.name} (${asset.tag}) has no published sha256 digest or checksum manifest`,
		};
		throw failure;
	}

	return {
		[RELEASE_ASSET_URL]: asset.url,
		[RELEASE_CHECKSUM]: asset.checksum,
	};
}

/**
 * Anything that crosses the IPC boundary can in principle arrive as something
 * other than the typed error Rust returns (a webview whose IPC is
 * unavailable throws a plain `Error`), so an unrecognised rejection is
 * reported as `offline` -- the kind whose message tells the user to check
 * their connection and try again, which is the right advice for every cause
 * this cannot name.
 */
function asResolveError(err: unknown): ReleaseResolveError {
	if (isReleaseResolveError(err)) return err;
	return { kind: 'offline', detail: String(err) };
}

/**
 * The `arrow.release.*` subset of `MessageKey`.
 *
 * Narrower than the full union on purpose: `t()` demands a params argument
 * for any key type that could plausibly take one, and none of these do, so
 * typing them as `MessageKey` would force every caller to pass an empty
 * object. Same reasoning as `ArrowActionLabelKey` in `actions.ts`.
 */
export type ReleaseMessageKey = Extract<MessageKey, `arrow.release.${string}`>;

const MESSAGE_KEYS: Record<ReleaseResolveErrorKind, ReleaseMessageKey> = {
	offline: 'arrow.release.offline',
	rate_limited: 'arrow.release.rateLimited',
	no_release: 'arrow.release.noRelease',
	http: 'arrow.release.unavailable',
	malformed: 'arrow.release.unavailable',
	no_asset: 'arrow.release.noAsset',
	unsupported_platform: 'arrow.release.unsupportedPlatform',
	unverifiable: 'arrow.release.unverifiable',
};

/**
 * The non-technical sentence for a resolution failure. An unknown `kind` (a
 * newer Rust side talking to an older bundle, which a partial update could
 * produce) falls back to the generic one rather than rendering a raw key.
 */
export function releaseErrorMessageKey(kind: string): ReleaseMessageKey {
	return MESSAGE_KEYS[kind as ReleaseResolveErrorKind] ?? 'arrow.release.unavailable';
}
