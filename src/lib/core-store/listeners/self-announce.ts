import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

const SELF_NAMESPACE = 'github.com/rabbytesoftware/quiver.desktop';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection -- the same `POST /v0/arrow/:ns` a user clicking "add" on any
 * other arrow would hit, at this app's own namespace.
 *
 * WHICH REF IT ANNOUNCES DEPENDS ON WHAT THIS BINARY IS, and both answers are
 * deliberate:
 *
 *   - A RELEASE BUILD announces `@<tag>`: the exact `stable-*` tag the build
 *     was cut from, baked in at compile time by `src-tauri/build.rs` and read
 *     back through `Backend.getBuildTag()` (see
 *     `src-tauri/src/commands/build_info.rs`). core's manifest resolver takes
 *     an explicit ref as written and does NOT fall back (resolvers/http.go:
 *     `if ref := namespace.Ref(); ref != "" { branches = []string{ref} }`), so
 *     this only works because the ref is real: `stable-release.yml` pushes the
 *     tag before it builds, and hands that same tag to the build. The row then
 *     tracks the version the user is ACTUALLY RUNNING.
 *   - ANY OTHER BUILD -- dev, PR CI, a local `tauri build`, anything not cut
 *     from a tag -- announces the namespace REFLESS, with no `@` at all. It
 *     has no ref it could honestly claim: an untagged commit is not a
 *     published ref, and `tauri.conf.json`'s `version` is a productVersion
 *     ("0.1.0"), never a git ref. Announcing that is what this call used to
 *     do, and it 404'd permanently rather than pending -- nothing in this
 *     repo's release process, which tags `stable-<series>[.patch]`, will ever
 *     create a ref named `0.1.0`.
 *
 * Refless is not a degraded fallback, it is core's own first-class answer.
 * `ResolveForInstall` reads a refless namespace through `resolveRefless`:
 * "the latest stable release, and a repository that publishes none as
 * whatever its default branch is" -- both read off the real remote, and the
 * namespace it returns always carries a concrete ref, so nothing refless ever
 * reaches the catalog. Today, with zero tags on quiver.desktop, that is the
 * default branch (`develop`, where this branch's ARROW.md lands) stamped
 * `RefIsBranch` + `RefCommitSHA`; once a real `stable-*` tag exists,
 * `resolveRefless` picks it and `checkBranchDrift` promotes an
 * already-registered branch-tracked row, trying tags first precisely so "a
 * branch is never again the answer" once a repository has any.
 *
 * WHAT THE REFLESS PATH STILL COSTS, honestly: such a row tracks the latest
 * PUBLISHED desktop release rather than the build in front of the user. The
 * outdated badge is generic (keyed off `ArrowState`, not namespace -- see
 * arrow-details/lib/status.ts), so the moment any newer commit lands on the
 * tracked branch/tag, this app's own tile shows the same "update available"
 * badge any other outdated arrow gets, above an Update action that is a no-op
 * because ARROW.md declares no `update` lifecycle. That was the accepted cost
 * of announcing refless unconditionally; stamping narrows it to builds that
 * are not releases -- a developer's own checkout, and CI -- where a tile
 * claiming to be behind `develop` is both true and nobody's problem. A user
 * running a real release now gets a row pinned to the tag they installed.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention (Rust's `.ok()`): a daemon too old for this
 * namespace, an unreachable manifest host, or a transient network error must
 * never fail or block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	const namespace = await selfNamespace();
	try {
		await apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, {
			method: 'POST',
		});
	} catch (err) {
		console.error('core-store: failed to self-announce quiver.desktop', err);
	}
}

/** This app's namespace, at its build tag when it has one. */
async function selfNamespace(): Promise<string> {
	const tag = await buildTag();
	return tag ? `${SELF_NAMESPACE}@${tag}` : SELF_NAMESPACE;
}

/**
 * Caught separately from the POST, and on purpose: a backend that cannot
 * answer (an old shell, a webview whose IPC is unavailable) must cost this
 * call its precision, not its existence. Refless still resolves; not
 * announcing at all leaves the app missing from its own catalog.
 */
async function buildTag(): Promise<string | null> {
	try {
		return await backend().getBuildTag();
	} catch (err) {
		console.error('core-store: could not read this build tag; announcing quiver.desktop refless', err);
		return null;
	}
}
