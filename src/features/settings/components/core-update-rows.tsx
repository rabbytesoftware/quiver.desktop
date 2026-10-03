import { Button } from '@/components/ui/button';

import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { useArrowUpdate } from '@/features/updates/hooks/use-arrow-update';
import { useLiveArrowDetail } from '@/features/updates/hooks/use-live-arrow-detail';
import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';
import { bareNamespace } from '@/lib/namespace';

import { SettingRow } from './section';

/**
 * The catalog identity quiver.core is filed under, which carries the channel it
 * follows. Empty until it has registered itself, which keeps the detail read
 * from guessing at a default row.
 */
function useCoreIdentity(): string {
	return useArrowStore((state) => {
		for (const namespace of state.arrows.keys()) {
			if (bareNamespace(namespace) === QUIVER_CORE_NAMESPACE) return namespace;
		}
		return '';
	});
}

// Each failure is already recorded by the hook and shown in the row, so the
// rejection it re-throws for other callers has nothing left to do here.
const run = (action: () => Promise<void>) => () => void action().catch(() => undefined);

export function CoreUpdateRows() {
	const { t } = useTranslation();
	const detail = useLiveArrowDetail(useCoreIdentity());
	const { installed, available, pending, state, error, checking, check, update, activate } = useArrowUpdate(detail);

	const retry = pending ? activate : available ? update : check;

	const version = pending?.version ?? available ?? '';
	function describe(): string {
		if (!detail) return t('settings.engine.selfUpdate.state.unknown');
		if (state !== 'error') return t(`settings.engine.selfUpdate.state.${state}`, { version });
		const kind = error?.kind ?? 'failed';
		return [t(`settings.engine.selfUpdate.error.${kind}`), error?.message].filter(Boolean).join(' ');
	}

	const busy = state === 'downloading' || state === 'restarting';

	return (
		<>
			<SettingRow label={t('settings.engine.selfUpdate.installed')}>
				<span className="font-mono text-xs text-muted-foreground">
					{installed || t('settings.engine.selfUpdate.unknown')}
				</span>
			</SettingRow>
			<SettingRow label={t('settings.engine.selfUpdate.available')}>
				<span className="font-mono text-xs text-muted-foreground">
					{available ?? t('settings.engine.selfUpdate.none')}
				</span>
			</SettingRow>
			<SettingRow label={t('settings.engine.selfUpdate.action')} description={describe()}>
				<Button variant="outline" size="sm" disabled={!detail || checking || busy} onClick={run(check)}>
					{checking ? t('settings.engine.selfUpdate.checking') : t('settings.engine.selfUpdate.check')}
				</Button>
				{state === 'available' && (
					<Button size="sm" onClick={run(update)}>
						{t('settings.engine.selfUpdate.update')}
					</Button>
				)}
				{state === 'downloading' && (
					<Button size="sm" disabled>
						{t('settings.engine.selfUpdate.updating')}
					</Button>
				)}
				{state === 'staged' && (
					<Button size="sm" onClick={run(activate)}>
						{t('settings.engine.selfUpdate.restartToApply')}
					</Button>
				)}
				{state === 'restarting' && (
					<Button size="sm" disabled>
						{t('settings.engine.selfUpdate.restarting')}
					</Button>
				)}
				{state === 'error' && (
					<Button size="sm" onClick={run(retry)}>
						{t('settings.engine.selfUpdate.retry')}
					</Button>
				)}
			</SettingRow>
		</>
	);
}
