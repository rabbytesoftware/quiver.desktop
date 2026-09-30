import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { namespaceSegment, withSelector } from '@/lib/namespace';
import { apiFetch, ApiError, apiRequest } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

import type { ArrowDetailDTO, ArrowListResponseItemDTO, ChannelListDTO } from '../dtos/v0/arrow';
import { useArrowStore } from '../store/arrows';

/** The channel a build tag belongs to: `src-tauri/build.rs` only ever stamps `stable-*`. */
const STABLE_CHANNEL = 'stable';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection, and makes its catalog row say what is actually running.
 *
 * 1. Register. A release build registers `quiver.desktop@stable`, the channel
 *    its `stable-*` build tag (stamped by `src-tauri/build.rs`) was cut on --
 *    a catalog identity's selector never changes, so the row follows the
 *    channel the way quiver.core files its own row. Any other build registers
 *    the repository's default channel -- the first one `/channels` lists,
 *    which is what a refless add would pick -- by name, so it knows which row
 *    is its own. Only when the channels cannot be read does it register
 *    refless, and then it forgets nothing. An identity already in the library
 *    is not registered again.
 * 2. Adopt. A release build then declares its own tag as what is installed
 *    (`POST /v0/arrow/:ns/adopt`), so the version check offers exactly the
 *    releases newer than the running one -- unless the row already records
 *    that tag, so re-announcing the same build sends nothing. A newer build
 *    (after a self-update relaunch) advances the same row in place. A tag
 *    core will not accept for the channel (400/404) leaves the plain
 *    registration.
 * 3. Forget stale pins. Earlier builds filed themselves as pins of their
 *    build tags. Only once the announce fully succeeded, every OTHER
 *    quiver.desktop row whose selector is a pin is removed; a channel
 *    identity is never touched, since it may be another build's own row.
 *
 * Fire-and-forget-with-logging throughout: a daemon too old for any of this,
 * an unreachable manifest host, or a transient error must never fail or
 * block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	const tag = await buildTag();
	const selector = tag ? STABLE_CHANNEL : await defaultChannel();
	const identity = withSelector(QUIVER_DESKTOP_NAMESPACE, selector);
	const known = await knownRows();

	if (!known.has(identity) && !(await register(identity))) return;
	if (tag && known.get(identity) !== tag && !(await adopt(identity, tag))) return;
	// A refless register leaves no way to tell which row core filed, so
	// nothing is forgotten on its account.
	if (selector) await forgetStalePins([...known.keys()], identity);
}

async function register(identity: string): Promise<boolean> {
	try {
		await apiRequest<void>(`/v0/arrow/${namespaceSegment(identity)}`, { method: 'POST' });
		return true;
	} catch (err) {
		console.error('core-store: failed to self-announce quiver.desktop', err);
		return false;
	}
}

/** The channel a refless add would file quiver.desktop under: the first `/channels` lists. `''` when unknown. */
async function defaultChannel(): Promise<string> {
	try {
		const { channels } = await apiFetch<ChannelListDTO>(
			`/v0/arrow/${namespaceSegment(QUIVER_DESKTOP_NAMESPACE)}/channels`
		);
		return channels[0]?.name ?? '';
	} catch (err) {
		console.error('core-store: could not read quiver.desktop channels; announcing refless', err);
		return '';
	}
}

/** Whether the declared state now stands: adopted, and the catalog asked to re-read what it records. */
async function adopt(identity: string, tag: string): Promise<boolean> {
	try {
		await apiFetch<void>(`/v0/arrow/${namespaceSegment(identity)}/adopt`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ resolved_ref: tag }),
		});
		useArrowStore.getState().refreshCatalog();
		return true;
	} catch (err) {
		if (err instanceof ApiError && (err.status === 400 || err.status === 404)) {
			console.debug(
				`core-store: ${tag} is not a release ${identity} can adopt; leaving the plain registration`,
				err
			);
		} else {
			console.error(`core-store: failed to adopt ${tag} for ${identity}`, err);
		}
		return false;
	}
}

/** Every quiver.desktop identity in the library and the ref each resolved to; none when the catalog cannot be read. */
async function knownRows(): Promise<Map<string, string>> {
	try {
		const items = await apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		const self = items.find((item) => item.namespace === QUIVER_DESKTOP_NAMESPACE);
		return new Map(
			(self?.versions ?? []).map((v) => [withSelector(QUIVER_DESKTOP_NAMESPACE, v.ref), v.resolved_ref ?? ''])
		);
	} catch (err) {
		console.error('core-store: could not list the catalog; leaving any stale quiver.desktop rows', err);
		return new Map();
	}
}

/** Whether core records `identity` as a pin -- what earlier builds filed. Unknown reads as not, so it is kept. */
async function isPin(identity: string): Promise<boolean> {
	try {
		const detail = await apiFetch<ArrowDetailDTO>(`/v0/arrow/${namespaceSegment(identity)}`);
		return (detail.selector_kind ?? 'pin') === 'pin';
	} catch (err) {
		console.error(`core-store: could not read ${identity}; keeping it`, err);
		return false;
	}
}

async function forgetStalePins(known: string[], current: string): Promise<void> {
	const removals = known.flatMap((stale) =>
		stale === current
			? []
			: [
					isPin(stale).then(async (pin) => {
						if (!pin) return;
						await apiFetch<void>(`/v0/arrow/${namespaceSegment(stale)}`, { method: 'DELETE' }).catch(
							(err: unknown) => {
								console.error(`core-store: could not remove the stale ${stale} row`, err);
							}
						);
					}),
				]
	);
	await Promise.all(removals);
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
