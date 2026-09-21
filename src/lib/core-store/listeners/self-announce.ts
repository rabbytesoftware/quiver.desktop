import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection -- the same `POST /v0/arrow/:ns` a user clicking "add" on any
 * other arrow would hit, at this app's own namespace.
 *
 * A release build announces `@<tag>`, the exact `stable-*` tag it was cut
 * from (stamped by `src-tauri/build.rs`, read via `Backend.getBuildTag()`),
 * since core's resolver takes an explicit ref as written and never falls
 * back. Any other build (dev, PR CI, a local `tauri build`) announces
 * refless: no ref it could honestly claim exists, and core's own
 * `resolveRefless` already answers "latest stable release, else the default
 * branch" -- refless is a first-class resolution there, not a degraded one.
 *
 * Advancing the row after an update is core's job, not this call's: a
 * successful `update:` execution triggers core's own `onUpdateEnded`
 * reaction (quiver.core, usecases/runtime.go), which swaps the row via the
 * same generic mechanism any other arrow's version bump uses. This call only
 * needs to run once per real version -- a re-announce at a tag core already
 * has on file is a no-op.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention: a daemon too old for this namespace, an
 * unreachable manifest host, or a transient network error must never fail
 * or block the connection this rides on.
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
	}
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
