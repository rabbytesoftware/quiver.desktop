import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';
import { namespaceSegment, withSelector } from '@/lib/namespace';
import { apiFetch, ApiError, apiRequest } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

import { announcedRows, type AnnouncedRows } from './announced-rows';
import type { ArrowListResponseItemDTO, ChannelListDTO } from '../dtos/v0/arrow';
import { useArrowStore } from '../store/arrows';

/** The channel a build tag belongs to: `src-tauri/build.rs` only ever stamps `stable-*`. */
const STABLE_CHANNEL = 'stable';

/**
 * Announces quiver.desktop to the connected daemon on every successful
 * connection, and makes its catalog row say what is actually running.
 *
 * 1. Register. A build registers the channel its release pipeline published
 *    it on (`VITE_QUIVER_BUILD_CHANNEL`: `stable`, `beta`, `hotfix`,
 *    `nightly-rolling`) -- a catalog identity's selector never changes, so the
 *    row follows that channel the way quiver.core files its own row. A
 *    channel the repository does not list yet is not registered. Without one,
 *    a `stable-*` build tag (stamped by `src-tauri/build.rs`) means `stable`,
 *    and any other build registers the repository's default channel -- the
 *    first one `/channels` lists, which is what a refless add would pick -- by
 *    name, so it knows which row is its own. Only when the channels cannot be
 *    read does it register refless, and then it forgets nothing. An identity
 *    already in the library is not registered again.
 * 2. Adopt. A release build then declares its own tag as what is installed
 *    (`POST /v0/arrow/:ns/adopt`), so the version check offers exactly the
 *    releases newer than the running one -- unless the row already records
 *    that tag, so re-announcing the same build sends nothing. A newer build
 *    (after a self-update relaunch) advances the same row in place. A tag
 *    core will not accept for the channel (400/404) leaves the plain
 *    registration.
 * 3. Forget what earlier announces left. An identity this app registered
 *    itself is recorded per connection (`announcedRows`); only once the
 *    announce fully succeeded, every OTHER recorded identity still in the
 *    library is removed -- the row a build of another channel filed, e.g.
 *    `@nightly-rolling` after the user moved to a stable build. A row the
 *    user added (any pin, any channel) was never recorded and is never
 *    touched.
 *
 * Fire-and-forget-with-logging throughout: a daemon too old for any of this,
 * an unreachable manifest host, or a transient error must never fail or
 * block the connection this rides on.
 */
export async function announceSelf(): Promise<void> {
	const tag = await buildTag();
	const selector = await selectorFor(tag);
	const identity = withSelector(QUIVER_DESKTOP_NAMESPACE, selector);
	const [known, record] = await Promise.all([knownRows(), announcedRows()]);

	if (!known?.has(identity)) {
		if (!(await register(identity))) return;
		// Only a row known to be absent is this app's own; a refless register
		// leaves no way to tell which row core filed.
		if (selector && known) record?.add(identity);
	}
	if (tag && known?.get(identity) !== tag && !(await adopt(identity, tag))) return;
	if (selector && known && record) await forgetEarlierAnnounces(record, known, identity);
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

/** The channel the release pipeline published this build on, or `''` for a local or PR build. */
function buildChannel(): string {
	return import.meta.env.VITE_QUIVER_BUILD_CHANNEL?.trim() ?? '';
}

/** The selector this build files itself under; `''` (refless) only when nothing names one. */
async function selectorFor(tag: string | null): Promise<string> {
	const channel = buildChannel();
	if (tag && !channel) return STABLE_CHANNEL;
	const listed = await listedChannels();
	if (channel) {
		if (listed === null || listed.includes(channel)) return channel;
		console.debug(`core-store: quiver.desktop publishes no ${channel} channel yet; announcing its default`);
	}
	if (tag) return STABLE_CHANNEL;
	return listed?.[0] ?? '';
}

/** Every channel quiver.desktop publishes, the one a refless add would pick first; `null` when unknown. */
async function listedChannels(): Promise<string[] | null> {
	try {
		const { channels } = await apiFetch<ChannelListDTO>(
			`/v0/arrow/${namespaceSegment(QUIVER_DESKTOP_NAMESPACE)}/channels`
		);
		return channels.map((c) => c.name);
	} catch (err) {
		console.error('core-store: could not read quiver.desktop channels; announcing refless', err);
		return null;
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

/** Every quiver.desktop identity in the library and the ref each resolved to; `null` when the catalog cannot be read. */
async function knownRows(): Promise<Map<string, string> | null> {
	try {
		const items = await apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true');
		const self = items.find((item) => item.namespace === QUIVER_DESKTOP_NAMESPACE);
		return new Map(
			(self?.versions ?? []).map((v) => [withSelector(QUIVER_DESKTOP_NAMESPACE, v.ref), v.resolved_ref ?? ''])
		);
	} catch (err) {
		console.error('core-store: could not list the catalog; leaving any stale quiver.desktop rows', err);
		return null;
	}
}

/** Removes every recorded identity but `current`; one the library no longer holds, or core no longer has, just leaves the record. */
async function forgetEarlierAnnounces(
	record: AnnouncedRows,
	known: Map<string, string>,
	current: string
): Promise<void> {
	const stale = record.list().filter((identity) => identity !== current);
	await Promise.all(
		stale.map(async (identity) => {
			if (!known.has(identity)) {
				record.drop(identity);
				return;
			}
			try {
				await apiFetch<void>(`/v0/arrow/${namespaceSegment(identity)}`, { method: 'DELETE' });
				record.drop(identity);
			} catch (err) {
				if (err instanceof ApiError && err.status === 404) {
					record.drop(identity);
					return;
				}
				console.error(
					`core-store: could not remove the stale ${identity} row; retrying on the next announce`,
					err
				);
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
