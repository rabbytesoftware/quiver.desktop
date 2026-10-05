import { useState, type JSX } from 'react';

import { InferredBadge } from '@/components/inferred-badge';
import { Badge } from '@/components/ui/badge';
import { FlickerSpinner } from '@/components/ui/flicker-spinner';

import type { ArrowDetail } from '@/domain/arrow';
import { isInferred, isPlatformSupported } from '@/domain/arrow';
import { computeActions } from '@/features/arrow-details/lib/actions';
import { CONTENT_MAX_WIDTH, CONTENT_PADDING_X } from '@/features/arrow-details/lib/layout';
import { problemMessage, computeStatus, STATUS_BADGE_VARIANT, STATUS_ICONS } from '@/features/arrow-details/lib/status';
import { useChannelSelection } from '@/features/arrow-details/lib/use-channel-selection';
import { useHeroActions } from '@/features/arrow-details/lib/use-hero-actions';
import { useSelectorSwitch } from '@/features/arrow-details/lib/use-selector-switch';
import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { cn } from '@/lib/cn';
import { cssUrl } from '@/lib/css';
import { useTranslation } from '@/lib/i18n';
import { withSelector } from '@/lib/namespace';

import { TriangleAlertIcon } from 'lucide-react';

import { ActionButton } from './action-button';
import { ChannelVersionSelects } from './channel-version-selects';
import { InferenceNotes } from './inference-notes';
import { MessageModal } from './message-modal';
import { OpenAppButton } from './open-app-button';
import { SelectorSummary } from './selector-summary';
import { SwitchSelectorDialog } from './switch-selector-dialog';

interface HeroProps {
	detail: ArrowDetail;
	platform: string;
	values: Record<string, string>;
	onValueChange: (name: string, value: string) => void;
	/** True while `GET /v0/arrow/:ns/channels` is still in flight -- the Channel/Version selects show their own loading state on this rather than waiting on it to render at all. Optional (defaults to `false`) so a caller with nothing to report about channels loading doesn't have to think about this. */
	channelsLoading?: boolean;
	/**
	 * True once `platform` is `useRealPlatform`'s resolved, authoritative
	 * value rather than its zero-latency UA guess. The not-supported
	 * indicator waits on this: rendering it off the guess risks a flicker at
	 * best and a false negative at worst on exactly the hardware (Apple
	 * Silicon Macs) this feature exists to get right for. Optional, default
	 * `true`, so a caller with no resolution concept of its own (a test, a
	 * future non-platform-aware host) sees today's behavior.
	 */
	platformResolved?: boolean;
	/** Moves the page to another identity of this arrow -- the row Add to Library or a switch just created. */
	onIdentityChange?: (namespace: string) => void;
}

/**
 * The arrow-details hero -- identity, status, tags, description, what the
 * entry follows + license, and the state-driven action row. Extends
 * Collection's existing hero pattern (banner + identity block) with
 * everything specific to a single arrow's lifecycle.
 *
 * What an entry follows is picked once, before it is in the library (the
 * Channel/Version selects choose the identity Add to Library registers).
 * After that it is read-only; following something else is the switch
 * dialog's uninstall + reinstall.
 */
export function Hero({
	detail,
	platform,
	values,
	onValueChange,
	channelsLoading = false,
	platformResolved = true,
	onIdentityChange,
}: HeroProps): JSX.Element {
	const { t } = useTranslation();
	const [problemOpen, setProblemOpen] = useState(false);
	const [switchOpen, setSwitchOpen] = useState(false);
	const channelSelection = useChannelSelection(detail.selector, detail.channels);
	const registerAs = channelSelection.selector
		? withSelector(detail.namespace, channelSelection.selector)
		: detail.namespace;
	const {
		pendingKind,
		releaseError,
		actionError,
		retryKind,
		upToDate,
		platformWarning,
		invoke,
		dismissReleaseError,
		dismissActionError,
		dismissPlatformWarning,
	} = useHeroActions(detail, values, registerAs, onIdentityChange);
	const selectorSwitch = useSelectorSwitch(detail, values, (namespace) => {
		setSwitchOpen(false);
		onIdentityChange?.(namespace);
	});
	const switchFailure = selectorSwitch.failure;
	const hasChannels = channelsLoading || detail.channels.length > 0;

	const status = computeStatus(detail);
	const problem = problemMessage(detail);
	// While a switch is working on this identity, its steps -- not the user --
	// drive install and uninstall here.
	const actions = computeActions(detail, platform).map((action) =>
		selectorSwitch.pending ? { ...action, forceDisabled: true } : action
	);
	// Always-visible, no click required to discover it -- unlike `problem`,
	// which needs an active run or a detached process, this can be true for an
	// arrow that has never been touched at all (the moment it's discovered).
	// Gated on platformResolved: rendering this off the UA guess risks a
	// flicker at best (the platform string changes mid-view once the real
	// value lands) and a false negative at worst (briefly claiming support
	// an arrow doesn't have) -- see use-real-platform.ts.
	const platformUnsupported = platformResolved && !isPlatformSupported(detail.targets, platform);

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
								{isInferred(detail) && <InferredBadge confidence={detail.confidence} focusable />}
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

					{isInferred(detail) && (
						<InferenceNotes confidence={detail.confidence} warnings={detail.warnings ?? []} />
					)}

					<div className="mt-2.5 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
						{detail.user_installed ? (
							<SelectorSummary
								detail={detail}
								onSwitch={detail.channels.length > 0 ? () => setSwitchOpen(true) : undefined}
								pending={selectorSwitch.pending}
							/>
						) : (
							<ChannelVersionSelects
								channels={detail.channels}
								channelsLoading={channelsLoading}
								disabled={pendingKind === 'addToLibrary'}
								selection={channelSelection}
							/>
						)}
						{detail.license && (
							<>
								{(detail.user_installed || hasChannels) && <span aria-hidden="true">–</span>}
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
						<OpenAppButton namespace={detail.namespace} surface={detail.active_run?.surface} />
						{upToDate && (
							<span className="text-xs text-muted-foreground" role="status">
								{t('arrow.update.current')}
							</span>
						)}
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
					onOpenChange={(open) => !open && dismissReleaseError()}
					open
					title={t('arrow.release.title')}
				/>
			)}

			{actionError &&
				(retryKind ? (
					<MessageModal
						cancelLabel={t('arrow.update.dismiss')}
						confirmLabel={t('arrow.update.retry')}
						message={`${t('arrow.update.busy')}\n\n${actionError}`}
						onConfirm={() => {
							dismissActionError();
							void invoke(retryKind);
						}}
						onOpenChange={(open) => !open && dismissActionError()}
						open
						title={t('arrow.action.error.title')}
					/>
				) : (
					<MessageModal
						message={actionError}
						onOpenChange={(open) => !open && dismissActionError()}
						open
						title={t('arrow.action.error.title')}
					/>
				))}

			{switchOpen && (
				<SwitchSelectorDialog
					channelsLoading={channelsLoading}
					detail={detail}
					error={selectorSwitch.registerError}
					onConfirm={(selector) => void selectorSwitch.switchTo(selector)}
					onOpenChange={(open) => {
						setSwitchOpen(open);
						if (!open) selectorSwitch.dismissRegisterError();
					}}
					pending={selectorSwitch.pending}
				/>
			)}

			{switchFailure &&
				(switchFailure.step === 'install' ? (
					<MessageModal
						message={`${t('arrow.selector.failed.install', switchFailure)}\n\n${switchFailure.reason}`}
						onOpenChange={(open) => !open && selectorSwitch.dismissFailure()}
						open
						title={t('arrow.selector.switchFailed')}
					/>
				) : (
					<MessageModal
						cancelLabel={t('arrow.selector.dismiss')}
						confirmLabel={t('arrow.selector.openOld')}
						message={`${t(`arrow.selector.failed.${switchFailure.step}`, switchFailure)}\n\n${switchFailure.reason}`}
						onConfirm={() => {
							selectorSwitch.dismissFailure();
							onIdentityChange?.(switchFailure.from);
						}}
						onOpenChange={(open) => !open && selectorSwitch.dismissFailure()}
						open
						title={t('arrow.selector.switchFailed')}
					/>
				))}

			{platformWarning !== null && (
				<MessageModal
					cancelLabel={t('arrow.platform.warning.cancel')}
					confirmLabel={t('arrow.platform.warning.confirm')}
					message={t('arrow.platform.warning.message', { platform: platformWarning })}
					onConfirm={() => {
						dismissPlatformWarning();
						void invoke('addToLibrary', true);
					}}
					onOpenChange={(open) => !open && dismissPlatformWarning()}
					open
					title={t('arrow.platform.warning.title')}
				/>
			)}
		</div>
	);
}
