import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import type { ArrowListResponseItemDTO } from '@/lib/core-store/dtos/v0/arrow';
import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

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
 * badge any other outdated arrow gets. ARROW.md now declares a real `update`
 * lifecycle (it did not when this comment was first written), so that badge
 * is actionable even on a refless row -- what stays imprecise is only WHICH
 * commit it is behind, not whether updating does anything. Stamping narrows
 * that imprecision to builds that are not releases -- a developer's own
 * checkout, and CI -- where a tile claiming to be behind `develop` is both
 * true and nobody's problem. A user running a real release gets a row pinned
 * to the exact tag they installed.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention (Rust's `.ok()`): a daemon too old for this
 * namespace, an unreachable manifest host, or a transient network error must
 * never fail or block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	const tag = await buildTag();
	const namespace = tag ? `${QUIVER_DESKTOP_NAMESPACE}@${tag}` : QUIVER_DESKTOP_NAMESPACE;
	try {
		await apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, {
			method: 'POST',
		});
	} catch (err) {
		console.error('core-store: failed to self-announce quiver.desktop', err);
		return;
	}
	if (tag) {
		await retireOtherSelfVersions(tag);
	}
}

/**
 * Removes every OTHER installed version of quiver.desktop's own catalog row
 * besides the one just announced.
 *
 * quiver.desktop is a single-instance app, not an ordinary package where
 * several installed versions can legitimately coexist side by side (see
 * quiver.core's own TestVersioning_TwoVersionsCoexist for the general case
 * this deliberately does not apply to): `update`'s lifecycle steps kill the
 * running process and overwrite its files in place, so there is never more
 * than one real install on disk. Without this, self-announcing at a new tag
 * after every update leaves the OLD tag's row behind forever -- nothing else
 * ever revisits it -- so it sits in the catalog permanently "installed" and
 * permanently outdated. quiver.core's own self-arrow has the identical
 * problem and solves it the same way, just from inside the daemon process in
 * Go (`selfarrow.RetireStale`) rather than over HTTP from here, since
 * quiver.desktop is a separate process with no access to call that directly.
 *
 * Only runs for a real release build (a known, exact tag, guaranteed by the
 * caller). A refless install (dev/CI) resolves through core's own
 * `resolveRefless` to whatever the daemon's manifold picks, and this process
 * has no reliable way to know which resolved ref that was without re-deriving
 * that same resolution itself, so it is left alone -- refless rows are
 * already a documented, lower-stakes cost (see the doc comment above).
 *
 * Fire-and-forget-with-logging, same as the announce itself: a listing or
 * deletion failure here must never surface to the caller or block startup.
 */
async function retireOtherSelfVersions(currentTag: string): Promise<void> {
	let items: ArrowListResponseItemDTO[];
	try {
		items = await apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
	} catch (err) {
		console.error('core-store: failed to list installed quiver.desktop versions for retirement', err);
		return;
	}
	const mine = items?.find((item) => item.namespace === QUIVER_DESKTOP_NAMESPACE);
	if (!mine) return;

	const stale = mine.versions.filter((v) => v.ref !== currentTag);
	await Promise.all(
		stale.map(async (v) => {
			const staleNamespace = `${QUIVER_DESKTOP_NAMESPACE}@${v.ref}`;
			try {
				await apiFetch<void>(`/v0/arrow/${encodeURIComponent(staleNamespace)}`, { method: 'DELETE' });
			} catch (err) {
				console.error(`core-store: failed to retire stale ${staleNamespace}`, err);
			}
		})
	);
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
