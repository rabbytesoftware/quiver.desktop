import { useSettingsUI } from '@/features/settings/store';
import { currentMock } from '@/lib/mock';
import { getScenario } from '@/lib/mock/world/scenarios';

/**
 * Reads `currentMock()`, not the store's `enabled` flag. `enabled` is an intent
 * settled on the next reload; this is what is answering right now. They
 * disagree across a reload, and whenever `installMock` caught a broken fixture
 * and fell back — where `enabled` is still true and the banner would be lying.
 */
export function MockIndicator() {
	const openSettings = useSettingsUI((s) => s.openSettings);
	const mock = currentMock();
	if (!mock) return null;

	const label = getScenario(mock.world.scenario).label;

	return (
		<div
			data-tauri-drag-region
			className="flex h-[22px] shrink-0 select-none items-center justify-center gap-2 bg-fill px-3 text-fill-ink"
		>
			<span className="text-[11px] font-medium uppercase tracking-[0.12em]">Mock</span>
			<span className="text-[11px] opacity-70">{label} · no daemon is being contacted</span>
			<button
				type="button"
				onClick={() => openSettings('developer')}
				className="ml-1 text-[11px] underline underline-offset-2 opacity-70 hover:opacity-100"
			>
				Turn off
			</button>
		</div>
	);
}
