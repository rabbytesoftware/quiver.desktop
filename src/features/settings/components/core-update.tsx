import { useQueryClient } from '@tanstack/react-query';

import { Button } from '@/components/ui/button';

import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { useAssembledArrowDetail } from '@/features/arrow-details/lib/use-assembled-arrow-detail';
import { useRecheckArrow, useUpdate } from '@/lib/core-store';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';
import { useTranslation } from '@/lib/i18n';
import { ApiError } from '@/lib/transport/api';

import { SettingRow } from './section';

// 422 is core refusing for a reason a second attempt will not change.
const isRetryable = (err: Error) => !(err instanceof ApiError) || err.status === 429 || err.status >= 500;

/**
 * quiver.core updates like any other arrow. Its update ends with the daemon
 * restarting on the same socket, which this panel does not model: the
 * connection machinery shows the drop, and the detail refetches on return.
 */
export function CoreUpdate() {
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const { detail } = useAssembledArrowDetail(QUIVER_CORE_NAMESPACE);
	const recheck = useRecheckArrow();
	const update = useUpdate();

	if (!detail) return null;

	const settle = () => queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
	const check = () => {
		update.reset();
		recheck.mutate({ namespace: detail.namespace }, { onSuccess: settle });
	};
	const run = () => {
		recheck.reset();
		update.mutate({ namespace: detail.namespace }, { onSuccess: settle });
	};

	const updating = detail.state === 'updating' || update.isPending;
	const failure = update.error ?? recheck.error;
	const description = failure ? failure.message : updating ? t('settings.engine.selfUpdate.restarts') : undefined;

	return (
		<>
			<SettingRow label={t('settings.engine.selfUpdate.installed')}>
				<span>{detail.resolved_ref || t('settings.engine.selfUpdate.unknown')}</span>
			</SettingRow>
			<SettingRow label={t('settings.engine.selfUpdate.available')}>
				<span>{detail.available?.ref ?? t('settings.engine.selfUpdate.current')}</span>
			</SettingRow>
			<SettingRow label={t('settings.engine.selfUpdate.check')} description={description}>
				{failure && isRetryable(failure) && (
					<Button variant="outline" size="sm" onClick={update.error ? run : check}>
						{t('arrow.update.retry')}
					</Button>
				)}
				<Button variant="outline" size="sm" disabled={updating || recheck.isPending} onClick={check}>
					{t('settings.engine.selfUpdate.check')}
				</Button>
				{detail.available && (
					<Button size="sm" disabled={updating} onClick={run}>
						{updating ? t('arrow.action.updating') : t('arrow.action.update')}
					</Button>
				)}
			</SettingRow>
		</>
	);
}
