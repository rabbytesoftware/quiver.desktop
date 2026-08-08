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

/**
 * A fixed instant for the format preview, not `Date.now()`.
 *
 * A live date would make the row's text change under the reader while they are
 * comparing languages, and would make any test of it a clock race. The point of
 * the preview is the SHAPE — 5 Mar 2026 against Mar 5, 2026 — which a fixed
 * date shows just as well.
 */
const PREVIEW_DATE = new Date(Date.UTC(2026, 2, 5, 14, 30));

/** Enough digits to show a grouping separator and a decimal separator at once. */
const PREVIEW_NUMBER = 1234567.89;

export function GeneralSettings() {
	const { t, formatDate, formatNumber } = useTranslation();
	const preference = useLocaleStore((s) => s.preference);
	const detected = useLocaleStore((s) => s.detected);
	const setPreference = useLocaleStore((s) => s.setPreference);

	const forced = localeForcedByEnv();

	/**
	 * Each language named IN ITSELF, read out of that language's own catalogue —
	 * `translateIn`, not `t`. Someone who has landed in a language they cannot
	 * read is looking for "Deutsch" in a list; "German", rendered in Japanese,
	 * is of no use to them at all. This is also why the picker is the one control
	 * in the app that must never be fully translated.
	 */
	const options: { value: LocalePreference; label: string }[] = [
		{ value: 'system', label: t('settings.general.language.system', { language: languageName(detected) }) },
		...LOCALES.map((locale) => ({ value: locale as LocalePreference, label: languageName(locale) })),
	];

	return (
		<div>
			<Section
				title={t('settings.general.language.title')}
				description={t('settings.general.language.description')}
			>
				<SettingRow
					label={t('settings.general.language.label')}
					description={
						forced
							? t('settings.general.language.forced', { language: languageName(forced) })
							: t('settings.general.language.hint')
					}
				>
					{/* `items` is what lets SelectValue render the label rather than
					    the raw tag — same reason as the scenario picker. */}
					<Select
						items={options}
						value={preference}
						// Disabled rather than live when the environment forces the
						// locale, for the reason the mock switch is: a live control
						// would write the preference, change nothing on screen, and
						// leave the user believing the setting is broken.
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

			<Section
				title={t('settings.general.formats.title')}
				description={t('settings.general.formats.description')}
			>
				<SettingRow label={t('settings.general.formats.date')}>
					{/* UTC so the preview cannot drift a day either side of midnight
					    depending on where the reader is. Real dates in the app use the
					    host zone, which is what a user wants for their own data. */}
					<span className="text-sm tabular-nums text-muted-foreground">
						{formatDate(PREVIEW_DATE, { timeZone: 'UTC' })}
					</span>
				</SettingRow>
				<SettingRow label={t('settings.general.formats.number')}>
					<span className="text-sm tabular-nums text-muted-foreground">{formatNumber(PREVIEW_NUMBER)}</span>
				</SettingRow>
			</Section>
		</div>
	);
}

function languageName(locale: (typeof LOCALES)[number]): string {
	return translateIn(locale, 'locale.name');
}
