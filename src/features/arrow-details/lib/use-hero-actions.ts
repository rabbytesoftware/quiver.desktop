import { useEffect, useRef, useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowDetail } from '@/domain/arrow';
import { isPlatformSupported } from '@/domain/arrow';
import type { ArrowActionKind } from '@/features/arrow-details/lib/actions';
import {
	isSelfArrow,
	releaseErrorMessageKey,
	installTagFor,
	releaseVariables,
	updateReleaseVariables,
	type ReleaseMessageKey,
} from '@/features/arrow-details/lib/release-variables';
import { resolveRealPlatform } from '@/features/arrow-details/lib/use-real-platform';
import { useArrowUpdate } from '@/features/updates/hooks/use-arrow-update';
import {
	useArrowStore,
	useExecuteArrow,
	useInstall,
	useRegisterArrow,
	useRemoveArrow,
	useStop,
	useUninstall,
	useUpdate,
} from '@/lib/core-store';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';
import { ApiError } from '@/lib/transport/api';

/**
 * A state violation (422) or conflict (409) on update is a race with another
 * update -- one underway, or the last one's commit still pending -- which a
 * retry a moment later settles. Every other failure is final as reported.
 */
function isRetryableUpdateError(err: unknown): boolean {
	return err instanceof ApiError && (err.status === 409 || err.status === 422);
}

export interface HeroActions {
	/** The action kind currently in flight, or null -- callers disable/spin the matching button on this. */
	pendingKind: ArrowActionKind | null;
	/** Set when resolving Quiver's own release asset fails, before any core call is made. */
	releaseError: { messageKey: ReleaseMessageKey; detail: string } | null;
	/** Set when a mutation itself rejects, carrying the backend's own precise reason. */
	actionError: string | null;
	/** Set alongside `actionError` when the failed action is worth simply trying again. */
	retryKind: ArrowActionKind | null;
	/** True after an Update that core answered with nothing newer: no run follows, so nothing is pending. */
	upToDate: boolean;
	/** Set to the platform `resolveRealPlatform()` decided against, opened instead of registering right away. */
	platformWarning: string | null;
	/** Runs the given action against core, gating `addToLibrary` on a fresh platform check unless `skipPlatformWarning`. */
	invoke(kind: ArrowActionKind, skipPlatformWarning?: boolean): Promise<void>;
	dismissReleaseError(): void;
	dismissActionError(): void;
	dismissPlatformWarning(): void;
}

const UP_TO_DATE_MS = 4000;

/**
 * The Hero's action-dispatch logic -- pulled out of `Hero` (react-doctor's
 * `no-giant-component`): what to call for each `ArrowActionKind`, the extra
 * variables Quiver's own self-arrow needs resolved fresh on every click, the
 * platform-unsupported confirmation gate, and restart's client-side
 * stop-then-execute sequencing.
 *
 * `registerAs` is the identity Add to Library files the arrow under -- the
 * page's own, or another selector of it picked in the Hero. When it differs,
 * `onIdentityChange` moves the page to the row that now exists.
 */
export function useHeroActions(
	detail: ArrowDetail,
	values: Record<string, string>,
	registerAs: string,
	onIdentityChange?: (namespace: string) => void
): HeroActions {
	const [pendingKind, setPendingKind] = useState<ArrowActionKind | null>(null);
	const [releaseError, setReleaseError] = useState<{ messageKey: ReleaseMessageKey; detail: string } | null>(null);
	const [actionError, setActionError] = useState<string | null>(null);
	const [retryKind, setRetryKind] = useState<ArrowActionKind | null>(null);
	const [upToDate, setUpToDate] = useState(false);
	const [platformWarning, setPlatformWarning] = useState<string | null>(null);
	const restarting = useRef(false);
	// Restart's second leg reads the namespace/values current as of the
	// moment `detail.state` actually reaches 'ready', not whatever the
	// effect closure captured when the request was first fired -- a ref
	// (stable identity, no re-render) keeps that read fresh without pulling
	// `detail.namespace`/`values` into the effect's own dependency array.
	// Updated in an effect (not during render) so the ref write stays out of
	// render's own purity contract; no dependency array is deliberate -- this
	// must resync after every render, not just when React decides to diff it.
	const latest = useRef({ namespace: detail.namespace, values });
	useEffect(() => {
		latest.current = { namespace: detail.namespace, values };
	});

	const queryClient = useQueryClient();
	const registerArrow = useRegisterArrow();
	const removeArrow = useRemoveArrow();
	const install = useInstall();
	const uninstall = useUninstall();
	const stop = useStop();
	const update = useUpdate();
	const execute = useExecuteArrow();
	const refreshCatalog = useArrowStore((state) => state.refreshCatalog);
	const arrowUpdate = useArrowUpdate(detail);

	useEffect(() => {
		if (!upToDate) return;
		const timer = setTimeout(() => setUpToDate(false), UP_TO_DATE_MS);
		return () => clearTimeout(timer);
	}, [upToDate]);

	/**
	 * The variables an action has to carry beyond whatever the user typed.
	 *
	 * For every arrow but Quiver's own this is nothing at all. For Quiver's
	 * own, `install`, `reinstall` and `update` fetch a release asset that
	 * `ARROW.md` cannot name (release filenames carry a static product
	 * version, and `${REF}` at update time is the version being left, not the
	 * one being installed), so the caller resolves it at the moment the button
	 * is clicked: an install from the release the row resolved to, an update
	 * from the release a fresh read says it moves to -- never from the
	 * selector it follows, and never an update from the installed ref.
	 *
	 * Resolved on EVERY click, never remembered. Nothing here may depend on a
	 * previous execution's values still being around inside core.
	 */
	async function extraVariables(kind: ArrowActionKind): Promise<Record<string, string>> {
		const needsRelease = kind === 'install' || kind === 'reinstall' || kind === 'update';
		if (!needsRelease || !isSelfArrow(detail.namespace)) return {};
		if (kind === 'update') return updateReleaseVariables(detail.namespace);
		return releaseVariables(installTagFor(detail));
	}

	async function invoke(kind: ArrowActionKind, skipPlatformWarning = false): Promise<void> {
		// Set before the (possibly async) platform check below, not after: the
		// button's own disabled-while-busy render is what stops a second click
		// from re-entering invoke() during that gap, the same guarantee every
		// other kind here already has by setting this synchronously up front.
		setPendingKind(kind);
		setUpToDate(false);
		setRetryKind(null);

		if (kind === 'addToLibrary' && !skipPlatformWarning) {
			// A fresh, authoritative read, not the platform prop: gating this
			// click on the reactive value risks the UA guess if the click lands
			// before useRealPlatform's effect has resolved, which would silently
			// skip the warning for a genuinely unsupported arrow rather than
			// merely mis-rendering a badge -- see resolveRealPlatform's own doc
			// comment.
			const real = await resolveRealPlatform();
			if (!isPlatformSupported(detail.targets, real)) {
				setPendingKind(null);
				setPlatformWarning(real);
				return;
			}
		}

		let release: Record<string, string>;
		try {
			release = await extraVariables(kind);
		} catch (err) {
			const { kind: errKind, detail: errDetail } = err as { kind: string; detail: string };
			setReleaseError({ messageKey: releaseErrorMessageKey(errKind), detail: errDetail });
			setPendingKind(null);
			// The page's `available` was stale; show what core holds now.
			if (errKind === 'nothing_ahead')
				await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
			return;
		}

		try {
			switch (kind) {
				case 'addToLibrary':
					await registerArrow.mutateAsync({ namespace: registerAs });
					// `user_installed` isn't part of the live WS-driven overlay (only
					// state/active_run/last_return are) -- the one-time detail fetch
					// needs an explicit refetch to pick up the new library membership.
					// Keyed by prefix, not the exact namespace: the mounted query may
					// be running under a bare namespace (Search's own links carry no
					// ref), which differs from the identity core filed it under.
					await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
					refreshCatalog();
					if (registerAs !== detail.namespace) onIdentityChange?.(registerAs);
					break;
				case 'removeFromLibrary':
					await removeArrow.mutateAsync({ namespace: detail.namespace });
					await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
					break;
				case 'install':
				case 'reinstall':
					await install.mutateAsync({
						namespace: detail.namespace,
						variables: { ...values, ...release },
					});
					break;
				case 'uninstall':
					await uninstall.mutateAsync({ namespace: detail.namespace });
					refreshCatalog();
					break;
				case 'update': {
					// `release` is empty for every arrow but Quiver's own, which
					// is the whole point: core requires only the variables the
					// method's own steps expand, and an ordinary arrow's update
					// expands none of these.
					const outcome = await update.mutateAsync({ namespace: detail.namespace, variables: release });
					// A 200 is a no-op: no run starts and no runtime event will
					// follow, so there is nothing to wait for -- only a detail
					// whose `available` was stale, re-read in place (the identity
					// never changes, so there is nowhere to navigate).
					if (outcome === 'current') {
						setUpToDate(true);
						await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
					}
					refreshCatalog();
					break;
				}
				case 'activate':
					await arrowUpdate.activate();
					break;
				case 'execute':
					await execute.mutateAsync({ namespace: detail.namespace, variables: values });
					break;
				case 'stop':
					await stop.mutateAsync({ namespace: detail.namespace });
					break;
				case 'restart':
					// Client-side sequencing, not a single core call. `pendingKind`
					// stays 'restart' through both legs -- cleared only by the effect
					// below, once `execute` itself has resolved (or the whole thing
					// has failed).
					restarting.current = true;
					await stop.mutateAsync({ namespace: detail.namespace });
					return;
			}
		} catch (err) {
			restarting.current = false;
			setPendingKind(null);
			setActionError(err instanceof Error ? err.message : String(err));
			if (kind === 'update' && isRetryableUpdateError(err)) setRetryKind('update');
			return;
		}
		setPendingKind(null);
	}

	// Restart's second leg: core only accepts `execute` once the arrow has
	// genuinely reached `ready` (not merely once the `stop` request was
	// accepted), so this waits for that live state transition rather than
	// firing immediately after `stop` resolves.
	useEffect(() => {
		if (!restarting.current || detail.state !== 'ready') return;
		restarting.current = false;
		execute
			.mutateAsync({ namespace: latest.current.namespace, variables: latest.current.values })
			.catch((err) => {
				setActionError(err instanceof Error ? err.message : String(err));
			})
			.finally(() => setPendingKind(null));
	}, [detail.state, execute]);

	return {
		// Activation outlives its own request: the daemon is gone until the new
		// version reports, so the button stays busy for all of it.
		pendingKind: arrowUpdate.state === 'restarting' ? 'activate' : pendingKind,
		releaseError,
		actionError,
		retryKind,
		upToDate,
		platformWarning,
		invoke,
		dismissReleaseError: () => setReleaseError(null),
		dismissActionError: () => {
			setActionError(null);
			setRetryKind(null);
		},
		dismissPlatformWarning: () => setPlatformWarning(null),
	};
}
