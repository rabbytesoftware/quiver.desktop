import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

import { useThemeStore, type ThemePreference } from '@/features/shell';
import type { SidebarSide } from '@/features/shell/geometry';
import { useShellStore } from '@/features/shell/store';
import {
	LOCALES,
	localeForcedByEnv,
	translateIn,
	useLocaleStore,
	useTranslation,
	type LocalePreference,
} from '@/lib/i18n';

import { Section, SettingRow } from '../section';

export function GeneralSettings() {
	const { t } = useTranslation();

	const theme = useThemeStore((s) => s.preference);
	const setTheme = useThemeStore((s) => s.setPreference);
	const side = useShellStore((s) => s.sidebarSide);
	const setSide = useShellStore((s) => s.setSidebarSide);

	const preference = useLocaleStore((s) => s.preference);
	const detected = useLocaleStore((s) => s.detected);
	const setPreference = useLocaleStore((s) => s.setPreference);
	const forced = localeForcedByEnv();

	const themes: { value: ThemePreference; label: string }[] = [
		{ value: 'system', label: t('settings.general.theme.system') },
		{ value: 'light', label: t('settings.general.theme.light') },
		{ value: 'dark', label: t('settings.general.theme.dark') },
	];

	const sides: { value: SidebarSide; label: string }[] = [
		{ value: 'left', label: t('settings.general.sidebar.left') },
		{ value: 'right', label: t('settings.general.sidebar.right') },
	];

	const locales: { value: LocalePreference; label: string }[] = [
		{ value: 'system', label: t('settings.general.language.system', { language: languageName(detected) }) },
		...LOCALES.map((locale) => ({ value: locale as LocalePreference, label: languageName(locale) })),
	];

	return (
		<div>
			<Section title={t('settings.general.appearance.title')}>
				<SettingRow
					label={t('settings.general.theme.label')}
					description={t('settings.general.theme.description')}
					onReset={() => setTheme('system')}
					canReset={theme !== 'system'}
				>
					<Select items={themes} value={theme} onValueChange={(v) => setTheme(v as ThemePreference)}>
						<SelectTrigger className="w-[132px]" aria-label={t('settings.general.theme.label')}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{themes.map((o) => (
								<SelectItem key={o.value} value={o.value}>
									{o.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</SettingRow>

				<SettingRow
					label={t('settings.general.sidebar.label')}
					description={t('settings.general.sidebar.description')}
					onReset={() => setSide('left')}
					canReset={side !== 'left'}
				>
					<Select items={sides} value={side} onValueChange={(v) => setSide(v as SidebarSide)}>
						<SelectTrigger className="w-[132px]" aria-label={t('settings.general.sidebar.label')}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{sides.map((o) => (
								<SelectItem key={o.value} value={o.value}>
									{o.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</SettingRow>
			</Section>

			<Section title={t('settings.general.language.title')}>
				<SettingRow
					label={t('settings.general.language.label')}
					description={
						forced
							? t('settings.general.language.forced', { language: languageName(forced) })
							: t('settings.general.language.hint')
					}
					onReset={() => setPreference('system')}
					canReset={preference !== 'system' && forced === null}
				>
					<Select
						items={locales}
						value={preference}
						disabled={forced !== null}
						onValueChange={(next) => setPreference(next as LocalePreference)}
					>
						<SelectTrigger className="w-[168px]" aria-label={t('settings.general.language.label')}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{locales.map((o) => (
								<SelectItem key={o.value} value={o.value}>
									{o.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</SettingRow>
			</Section>
		</div>
	);
}

function languageName(locale: (typeof LOCALES)[number]): string {
	return translateIn(locale, 'locale.name');
}
