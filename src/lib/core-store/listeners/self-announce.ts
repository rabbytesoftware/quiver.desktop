import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { namespaceSegment, withSelector } from '@/lib/namespace';
import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

/** The one channel a build tag names: `src-tauri/build.rs` only ever stamps `stable-*`. */
const STABLE_CHANNEL = 'stable';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection -- the same `POST /v0/arrow/:ns` a user clicking "add" on any
 * other arrow would hit, at this app's own namespace.
 *
 * A release build announces `@stable`, the channel its `stable-*` build tag
 * (stamped by `src-tauri/build.rs`, read via `Backend.getBuildTag()`) was cut
 * on. A catalog identity's selector never changes, so the row has to follow
 * the channel -- the build tag as a pin never moves, and a row pinned to it
 * would never report a newer release. quiver.core files its own row the same
 * way (`quiver.core@<channel>`). Any other build (dev, PR CI, a local
 * `tauri build`) announces refless, and core files it under the repository's
 * default channel.
 *
 * Moving the row after an update is core's job, not this call's: the
 * runtime update advances the same identity in place. Re-announcing an
 * identity core already has is a no-op.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention: a daemon too old for this namespace, an
 * unreachable manifest host, or a transient network error must never fail
 * or block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	const tag = await buildTag();
	const namespace = withSelector(QUIVER_DESKTOP_NAMESPACE, tag ? STABLE_CHANNEL : '');
	try {
		await apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}`, {
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
