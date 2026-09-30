import { useEffect } from 'react';

import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';

import { pathState } from '@/features/settings/lib/path-state';
import { usePathStore } from '@/features/settings/stores/path-store';
import { useTranslation } from '@/lib/i18n';

import { Notice, Section, SettingRow } from './section';

export function PathSettings() {
	const { t } = useTranslation();
	const status = usePathStore((s) => s.status);
	const loading = usePathStore((s) => s.loading);
	const settingUp = usePathStore((s) => s.settingUp);
	const unavailable = usePathStore((s) => s.unavailable);
	const error = usePathStore((s) => s.error);
	const setupError = usePathStore((s) => s.setupError);
	const load = usePathStore((s) => s.load);
	const setup = usePathStore((s) => s.setup);

	useEffect(() => {
		void load();
	}, [load]);

	if (unavailable) return null;

	const title = t('settings.engine.path.title');
	const label = t('settings.engine.path.label');

	if (error) {
		return (
			<Section title={title}>
				<SettingRow label={label} description={error}>
					<Button variant="outline" size="sm" onClick={() => void load()}>
						{t('settings.engine.retry')}
					</Button>
				</SettingRow>
			</Section>
		);
	}

	if (!status) {
		return (
			<Section title={title}>
				<SettingRow label={label} description={loading ? t('settings.engine.path.loading') : undefined}>
					<Spinner className="size-4" />
				</SettingRow>
			</Section>
		);
	}

	const state = pathState(status);
	const files = status.files.length > 0 ? t('settings.engine.path.files', { files: status.files.join(', ') }) : null;
	const description = [t(`settings.engine.path.description.${state}`, { dir: status.binDir }), files]
		.filter(Boolean)
		.join(' ');

	return (
		<Section title={title}>
			{setupError && <Notice tone="error">{setupError}</Notice>}
			<SettingRow label={label} description={description}>
				{state === 'setup' ? (
					<Button size="sm" disabled={settingUp} onClick={() => void setup()}>
						{t('settings.engine.path.setup')}
					</Button>
				) : (
					<span className="text-xs text-muted-foreground">{t(`settings.engine.path.state.${state}`)}</span>
				)}
			</SettingRow>
		</Section>
	);
}
