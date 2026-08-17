import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

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
	const preference = useLocaleStore((s) => s.preference);
	const detected = useLocaleStore((s) => s.detected);
	const setPreference = useLocaleStore((s) => s.setPreference);

	const forced = localeForcedByEnv();

	const options: { value: LocalePreference; label: string }[] = [
		{ value: 'system', label: t('settings.general.language.system', { language: languageName(detected) }) },
		...LOCALES.map((locale) => ({ value: locale as LocalePreference, label: languageName(locale) })),
	];

	return (
		<Section title={t('settings.general.language.title')}>
			<SettingRow
				label={t('settings.general.language.label')}
				description={
					forced
						? t('settings.general.language.forced', { language: languageName(forced) })
						: t('settings.general.language.hint')
				}
			>
				<Select
					items={options}
					value={preference}
					disabled={forced !== null}
					onValueChange={(next) => setPreference(next as LocalePreference)}
				>
					<SelectTrigger className="w-[168px]" aria-label={t('settings.general.language.label')}>
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{options.map((option) => (
							<SelectItem key={option.value} value={option.value}>
								{option.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</SettingRow>
		</Section>
	);
}

function languageName(locale: (typeof LOCALES)[number]): string {
	return translateIn(locale, 'locale.name');
}
