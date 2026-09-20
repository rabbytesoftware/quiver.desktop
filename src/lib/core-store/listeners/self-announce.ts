import { apiFetch } from '@/lib/transport/api';

const SELF_NAMESPACE = 'github.com/rabbytesoftware/quiver.desktop';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection -- the same `POST /v0/arrow/:ns` a user clicking "add" on any
 * other arrow would hit, at this app's own namespace.
 *
 * DELIBERATELY REFLESS -- no `@<version>` suffix, and that is the whole point
 * rather than an omission.
 *
 * This used to announce `@${Backend.getAppVersion()}`, which is Tauri's
 * `getVersion()`, which is literally tauri.conf.json's `"version": "0.1.0"`.
 * quiver.core's manifest resolver takes an explicit ref as written and does
 * NOT fall back (resolvers/http.go: `if ref := namespace.Ref(); ref != "" {
 * branches = []string{ref} }`), so that required a git ref named `0.1.0` to
 * exist on the repo. Nothing creates one: quiver.desktop has no tags at all
 * today, and stable-release.yml names its tags `stable-<series>[.patch]`. The
 * announced ref and the real tagging convention could never agree, so the
 * call 404'd permanently -- not merely until this branch merged.
 *
 * Refless is not a degraded fallback, it is core's own first-class answer.
 * `ResolveForInstall` reads a refless namespace through `resolveRefless`:
 * "the latest stable release, and a repository that publishes none as
 * whatever its default branch is" -- both read off the real remote, and the
 * namespace it returns always carries a concrete ref, so nothing refless ever
 * reaches the catalog. That gives exactly the `stable-*`-shaped ref the rest
 * of this system uses, computed by core from the actual published tags, with
 * no build-time version stamping on this side at all:
 *
 *   - today, with zero tags on quiver.desktop, it resolves to the repo's
 *     default branch (`develop`) -- where this branch's ARROW.md lands -- and
 *     core stamps the row `RefIsBranch` + `RefCommitSHA`;
 *   - the moment a real `stable-*` tag exists, `resolveRefless` picks it, and
 *     an already-registered branch-tracked row is promoted by
 *     `checkBranchDrift`, which tries tags first precisely so "a branch is
 *     never again the answer" once a repository has any.
 *
 * The accepted trade-off: the row tracks the latest PUBLISHED desktop release
 * rather than the exact build the user is running -- and this DOES cost
 * something, not nothing. The outdated badge is generic (keyed off
 * `ArrowState`, not namespace -- see arrow-details/lib/status.ts), so once
 * ANY newer commit lands on the tracked branch/tag, this app's own tile shows
 * the same "update available" badge any other outdated arrow gets. Its
 * Update action is currently a no-op, since ARROW.md declares no `update`
 * lifecycle. This is a known, accepted cosmetic gap (real but non-damaging),
 * not a defect in this file -- it closes once quiver.desktop gains its own
 * build-time version stamping and can announce a concrete ref again instead
 * of refless. See the final-review-fix-desktop report for the full trade-off
 * discussion.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention (Rust's `.ok()`): a daemon too old for this
 * namespace, an unreachable manifest host, or a transient network error must
 * never fail or block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	try {
		await apiFetch<void>(`/v0/arrow/${encodeURIComponent(SELF_NAMESPACE)}`, {
			method: 'POST',
		});
	} catch (err) {
		console.error('core-store: failed to self-announce quiver.desktop', err);
	}
}
