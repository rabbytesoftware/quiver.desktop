import { useEffect, useRef, useState, type JSX } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import { Badge } from '@/components/ui/badge';
import { FlickerSpinner } from '@/components/ui/flicker-spinner';

import type { ArrowDetail } from '@/domain/arrow';
import { isPlatformSupported } from '@/domain/arrow';
import { computeActions, type ArrowActionKind } from '@/features/arrow-details/lib/actions';
import { CONTENT_MAX_WIDTH, CONTENT_PADDING_X } from '@/features/arrow-details/lib/layout';
import {
	isSelfArrow,
	releaseErrorMessageKey,
	releaseVariables,
	type ReleaseMessageKey,
} from '@/features/arrow-details/lib/release-variables';
import { problemMessage, computeStatus, STATUS_BADGE_VARIANT, STATUS_ICONS } from '@/features/arrow-details/lib/status';
import { useChannelSelection } from '@/features/arrow-details/lib/use-channel-selection';
import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { cn } from '@/lib/cn';
import {
	useExecuteArrow,
	useInstall,
	useRegisterArrow,
	useRemoveArrow,
	useStop,
	useUninstall,
	useUpdate,
} from '@/lib/core-store';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';
import { cssUrl } from '@/lib/css';
import { useTranslation } from '@/lib/i18n';

import { TriangleAlertIcon } from 'lucide-react';

import { ActionButton } from './action-button';
import { ChannelVersionSelects } from './channel-version-selects';
import { MessageModal } from './message-modal';

interface HeroProps {
	detail: ArrowDetail;
	platform: string;
	values: Record<string, string>;
	onValueChange: (name: string, value: string) => void;
	/** True while `GET /v0/arrow/:ns/channels` is still in flight -- the Channel/Version selects show their own loading state on this rather than waiting on it to render at all. Optional (defaults to `false`) so a caller with nothing to report about channels loading doesn't have to think about this. */
	channelsLoading?: boolean;
}

/**
 * The arrow-details hero -- identity, status, tags, description, channel +
 * version + license, and the state-driven action row. Extends Collection's
 * existing hero pattern (banner + identity block) with everything specific
 * to a single arrow's lifecycle.
 */
export function Hero({ detail, platform, values, onValueChange, channelsLoading = false }: HeroProps): JSX.Element {
	const { t } = useTranslation();
	const [problemOpen, setProblemOpen] = useState(false);
	const [pendingKind, setPendingKind] = useState<ArrowActionKind | null>(null);
	// Set when resolving Quiver's own release asset fails, which happens
	// before any core call is made -- so there is no `last_return` for the
	// generic problem chip to read, and nothing would otherwise appear on
	// screen. Shown through the same MessageModal the problem chip opens.
	const [releaseError, setReleaseError] = useState<{ messageKey: ReleaseMessageKey; detail: string } | null>(null);
	// Set when a mutation itself rejects -- `ApiError` from a non-2xx core
	// response carries the backend's own precise reason in `.message`, and it
	// used to be dropped on the floor by a bare `catch { ... }` that reset
	// only the busy-state bookkeeping below. Shown through the same
	// MessageModal the problem chip and releaseError above use.
	const [actionError, setActionError] = useState<string | null>(null);
	// Gates the "Add to Library" click for an arrow with no build for this
	// platform -- opened instead of registering right away; confirming re-runs
	// `invoke('addToLibrary', true)`, the `true` bypassing this same check so
	// the confirmed click isn't warned about a second time.
	const [platformWarningOpen, setPlatformWarningOpen] = useState(false);
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
	const channelSelection = useChannelSelection(detail);

	const status = computeStatus(detail);
	const problem = problemMessage(detail);
	const actions = computeActions(detail, platform);
	// Always-visible, no click required to discover it -- unlike `problem`,
	// which needs an active run or a detached process, this can be true for an
	// arrow that has never been touched at all (the moment it's discovered).
	const platformUnsupported = !isPlatformSupported(detail.targets, platform);

	/**
	 * The variables an action has to carry beyond whatever the user typed.
	 *
	 * For every arrow but Quiver's own this is nothing at all. For Quiver's
	 * own, `install`, `reinstall` and `update` fetch a release asset that
	 * `ARROW.md` cannot name (release filenames carry a static product
	 * version, and `${REF}` at update time is the version being left, not the
	 * one being installed), so the caller has to resolve it -- which is
	 * exactly what `install.sh` does against the same releases API, and what
	 * this does at the moment the button is clicked.
	 *
	 * Resolved on EVERY click, never remembered. Nothing here may depend on a
	 * previous execution's values still being around inside core.
	 */
	async function extraVariables(kind: ArrowActionKind): Promise<Record<string, string>> {
		const needsRelease = kind === 'install' || kind === 'reinstall' || kind === 'update';
		if (!needsRelease || !isSelfArrow(detail.namespace)) return {};
		return releaseVariables();
	}

	async function invoke(kind: ArrowActionKind, skipPlatformWarning = false) {
		if (kind === 'addToLibrary' && !skipPlatformWarning && !isPlatformSupported(detail.targets, platform)) {
			setPlatformWarningOpen(true);
			return;
		}

		setPendingKind(kind);
		let release: Record<string, string>;
		try {
			release = await extraVariables(kind);
		} catch (err) {
			const { kind: errKind, detail: errDetail } = err as { kind: string; detail: string };
			setReleaseError({ messageKey: releaseErrorMessageKey(errKind), detail: errDetail });
			setPendingKind(null);
			return;
		}

		try {
			switch (kind) {
				case 'addToLibrary':
					await registerArrow.mutateAsync({
						namespace: detail.namespace,
						...(channelSelection.selectedChannel ? { channel: channelSelection.selectedChannel } : {}),
					});
					// `user_installed` isn't part of the live WS-driven overlay (only
					// state/active_run/last_return are) -- the one-time detail fetch
					// needs an explicit refetch to pick up the new library membership.
					// Keyed by prefix, not the exact namespace: the mounted query may
					// be running under a bare namespace (Search's own links carry no
					// ref), which differs from this ref-qualified `detail.namespace`.
					await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
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
					break;
				case 'update':
					// `release` is empty for every arrow but Quiver's own, which
					// is the whole point: core requires only the variables the
					// method's own steps expand, and an ordinary arrow's update
					// expands none of these.
					await update.mutateAsync({ namespace: detail.namespace, variables: release });
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

	const StatusIcon = STATUS_ICONS[status.iconKind];
	const banner = detail.media.banner;

	return (
		<div className={cn(CONTENT_PADDING_X, CONTENT_MAX_WIDTH, 'py-6')}>
			<div
				className={cn('grid gap-6', banner ? 'grid-cols-1 lg:grid-cols-[440px_minmax(0,1fr)]' : 'grid-cols-1')}
			>
				{banner && (
					<div
						className="aspect-2/1 w-full overflow-hidden rounded-lg bg-muted bg-contain bg-center bg-no-repeat"
						style={{ backgroundImage: cssUrl(banner) }}
					/>
				)}

				<div className="flex min-h-0 flex-col">
					<div className="flex items-center gap-4">
						<span className="shrink-0" style={{ '--icon': '44px' } as React.CSSProperties}>
							<ArrowIcon icon={detail.media.icon} name={detail.name} namespace={detail.namespace} />
						</span>
						<div className="min-w-0 flex-1">
							<div className="flex min-w-0 items-center gap-2">
								<h1 className="truncate text-lg font-semibold tracking-tight">{detail.name}</h1>
								<Badge className="shrink-0 gap-1" variant={STATUS_BADGE_VARIANT[status.iconKind]}>
									{status.iconKind === 'busy' ? (
										<FlickerSpinner aria-hidden="true" className="size-3" />
									) : (
										<StatusIcon aria-hidden="true" />
									)}
									{t(status.labelKey)}
								</Badge>
								{platformUnsupported && (
									<Badge className="shrink-0 gap-1" variant="error">
										<TriangleAlertIcon aria-hidden="true" className="size-3" />
										{t('arrow.platform.unsupported', { platform })}
									</Badge>
								)}
							</div>
							<p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
								{detail.namespace}
							</p>
						</div>
					</div>

					{detail.tags.length > 0 || problem ? (
						<div className="mt-3 flex flex-wrap items-center gap-2">
							{detail.tags.map((tag) => (
								<Badge key={tag} variant="outline">
									{tag}
								</Badge>
							))}
							{problem && (
								<Badge
									className="cursor-pointer gap-1"
									onClick={() => setProblemOpen(true)}
									render={<button type="button" />}
									variant="error"
								>
									<TriangleAlertIcon aria-hidden="true" className="size-3" />
									{t('arrow.problem.label')}
								</Badge>
							)}
						</div>
					) : null}

					<p className="mt-3 line-clamp-2 max-w-2xl text-sm text-muted-foreground">{detail.description}</p>

					<div className="mt-2.5 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
						<ChannelVersionSelects
							channels={detail.channels}
							channelsLoading={channelsLoading}
							selection={channelSelection}
						/>
						{detail.license && (
							<>
								{(channelsLoading || detail.channels.length > 0) && <span aria-hidden="true">–</span>}
								<span>{detail.license}</span>
							</>
						)}
					</div>

					<div className="mt-auto flex flex-wrap items-center gap-2.5 pt-4">
						{actions.map((action) => (
							<ActionButton
								action={action}
								key={action.kind}
								onInvoke={() => invoke(action.kind)}
								onValueChange={onValueChange}
								pending={pendingKind === action.kind}
								values={values}
								variables={detail.variables}
							/>
						))}
					</div>
				</div>
			</div>

			{problem && (
				<MessageModal
					message={
						problem.reason === 'detached'
							? t('arrow.problem.detachedNote')
							: (problem.detail ?? t('arrow.problem.failedNote'))
					}
					onOpenChange={setProblemOpen}
					open={problemOpen}
					title={t('arrow.problem.label')}
				/>
			)}

			{releaseError && (
				<MessageModal
					// The sentence first, then the wire text underneath it, the
					// same shape the problem dialog uses for a failed step: a
					// person reads the first line and stops, and anyone
					// reporting the issue has the specifics without having to
					// find a log.
					message={`${t(releaseError.messageKey)}\n\n${releaseError.detail}`}
					onOpenChange={(open) => !open && setReleaseError(null)}
					open
					title={t('arrow.release.title')}
				/>
			)}

			{actionError && (
				<MessageModal
					message={actionError}
					onOpenChange={(open) => !open && setActionError(null)}
					open
					title={t('arrow.action.error.title')}
				/>
			)}

			{platformWarningOpen && (
				<MessageModal
					cancelLabel={t('arrow.platform.warning.cancel')}
					confirmLabel={t('arrow.platform.warning.confirm')}
					message={t('arrow.platform.warning.message', { platform })}
					onConfirm={() => {
						setPlatformWarningOpen(false);
						void invoke('addToLibrary', true);
					}}
					onOpenChange={(open) => !open && setPlatformWarningOpen(false)}
					open
					title={t('arrow.platform.warning.title')}
				/>
			)}
		</div>
	);
}
