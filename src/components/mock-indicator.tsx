import { useSettingsUI } from '@/features/settings/store';
import { useTranslation } from '@/lib/i18n';
import { currentMock } from '@/lib/mock';
import { getScenario } from '@/lib/mock/world/scenarios';

/**
 * Reads `currentMock()`, not the store's `enabled` flag. `enabled` is an intent
 * settled on the next reload; this is what is answering right now. They
 * disagree across a reload, and whenever `installMock` caught a broken fixture
 * and fell back — where `enabled` is still true and the banner would be lying.
 */
export function MockIndicator() {
	const { t } = useTranslation();
	const openSettings = useSettingsUI((s) => s.openSettings);
	const mock = currentMock();
	if (!mock) return null;

	// Interpolated rather than concatenated, so a language that puts the
	// qualifier first can say so in its own catalogue. English "Extreme · no
	// daemon…" and its reverse are the same key with a different template.
	const label = getScenario(mock.world.scenario).label;

	return (
		<div
			data-tauri-drag-region
			className="flex h-[22px] shrink-0 select-none items-center justify-center gap-2 bg-primary px-3 text-primary-foreground"
		>
			<span className="text-[11px] font-medium uppercase tracking-[0.12em]">{t('mock.badge')}</span>
			<span className="text-[11px] opacity-70">{t('mock.status', { scenario: label })}</span>
			<button
				type="button"
				onClick={() => openSettings('developer')}
				className="ml-1 text-[11px] underline underline-offset-2 opacity-70 hover:opacity-100"
			>
				{t('mock.turnOff')}
			</button>
		</div>
	);
}
