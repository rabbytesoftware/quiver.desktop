import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

const SELF_NAMESPACE = 'github.com/rabbytesoftware/quiver.desktop';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection -- the same `POST /v0/arrow/:ns` a user clicking "add" on any
 * other arrow would hit, at this app's own namespace and running version.
 *
 * quiver.desktop's own ARROW.md (repo root) declares a `preinstalled` check,
 * so this call is self-verifying: quiver.core runs that check at `Add` time
 * and only marks the row Ready if it genuinely finds the app present --
 * this call never claims to be installed on its own say-so.
 *
 * Fire-and-forget-with-logging, matching `emit_core_status`'s
 * swallow-on-failure convention (Rust's `.ok()`): a daemon too old for this
 * namespace, an unreachable manifest host, or a transient network error must
 * never fail or block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	try {
		const version = await backend().getAppVersion();
		await apiFetch<void>(`/v0/arrow/${encodeURIComponent(`${SELF_NAMESPACE}@${version}`)}`, {
			method: 'POST',
		});
	} catch (err) {
		console.error('core-store: failed to self-announce quiver.desktop', err);
	}
}
