import type { ArrowDetail } from '@/domain/arrow';
import type { ReleaseResolveError, ReleaseResolveErrorKind } from '@/domain/release';
import { isReleaseResolveError, QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import type { ArrowDetailDTO } from '@/lib/core-store/dtos/v0/arrow';
import type { MessageKey } from '@/lib/i18n';
import { namespaceSegment } from '@/lib/namespace';
import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

/** The two variables `ARROW.md` declares without defaults. */
export const RELEASE_ASSET_URL = 'QUIVER_RELEASE_ASSET_URL';
export const RELEASE_CHECKSUM = 'QUIVER_RELEASE_CHECKSUM';

export const RELEASE_VARIABLE_NAMES: readonly string[] = [RELEASE_ASSET_URL, RELEASE_CHECKSUM];

/**
 * Whether this arrow is Quiver itself.
 *
 * The resolver reads quiver.desktop's OWN releases, so it is only ever
 * correct for quiver.desktop's own row, whichever selector that row follows.
 */
export function isSelfArrow(namespace: string): boolean {
	return namespace.split('@')[0] === QUIVER_DESKTOP_NAMESPACE;
}

/**
 * The release an install of this row puts on disk: the ref it resolved to.
 * Never the identity selector -- `stable` or `v1.*` names what the row
 * follows, not a release.
 */
export function installTagFor(detail: Pick<ArrowDetail, 'resolved_ref'>): string | undefined {
	return detail.resolved_ref || undefined;
}

/**
 * The release variables for updating `namespace`, from the release the row
 * moves to -- read fresh from core at click time, because the page's own
 * `available` comes from a one-time read and the version check can move it
 * while the page is open. Refuses (`nothing_ahead`) when core reports nothing
 * newer: falling back to the installed ref would reinstall the old release
 * while core advances the row to the new one.
 */
export async function updateReleaseVariables(namespace: string): Promise<Record<string, string>> {
	const detail = await apiFetch<ArrowDetailDTO>(`/v0/arrow/${namespaceSegment(namespace)}`).catch((err: unknown) => {
		throw asResolveError(err);
	});
	const target = detail?.available?.ref;
	if (!target) {
		const failure: ReleaseResolveError = {
			kind: 'nothing_ahead',
			detail: `${namespace} has nothing newer to update to`,
		};
		throw failure;
	}
	return releaseVariables(target);
}

/**
 * Resolves this app's own release asset, from the release tagged `tag` (the
 * newest release when there is none), into the two execution variables
 * quiver.core requires for `install` and `update`.
 *
 * Runs at click time rather than being baked into the manifest: the asset
 * isn't derivable from anything the manifest knows, and `${REF}` during an
 * update names the version being left, not the one being installed. See
 * `src-tauri/src/release/mod.rs`.
 *
 * Resolved fresh on every call, never cached: quiver.core's variable layer
 * used to carry a previous execution's answer forward, so a bare re-update
 * could silently re-install the LAST update's asset instead of failing.
 * That layer no longer does, and this side never relies on it either.
 *
 * Does not inherit `install.sh`'s "warn and install anyway" fallback for an
 * unverifiable release: `ARROW.md` verifies the checksum variable, and
 * quiver.core's fetch step refuses one that resolved to empty rather than
 * treating it as "skip verification" -- offering that here would promise
 * something that fails mid-update with the app already killed. Both this
 * and the installers now read GitHub's per-asset digest as well as a
 * published checksum manifest, so both verify whenever anything is
 * publishable at all.
 */
export async function releaseVariables(tag?: string): Promise<Record<string, string>> {
	const asset = await backend()
		.resolveReleaseAsset(tag)
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
	nothing_ahead: 'arrow.release.nothingAhead',
};

/**
 * The non-technical sentence for a resolution failure. An unknown `kind` (a
 * newer Rust side talking to an older bundle, which a partial update could
 * produce) falls back to the generic one rather than rendering a raw key.
 */
export function releaseErrorMessageKey(kind: string): ReleaseMessageKey {
	return MESSAGE_KEYS[kind as ReleaseResolveErrorKind] ?? 'arrow.release.unavailable';
}
