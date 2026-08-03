import { useSettingsUI } from '@/features/settings/store';
import { currentMock } from '@/lib/mock';
import { getScenario } from '@/lib/mock/world/scenarios';

/**
 * Says, permanently and unmissably, that nothing on screen is real.
 *
 * This ships in release builds, which turns "all my arrows disappeared" from a
 * hypothetical into a failure mode with a plausible path: someone unlocks the
 * tab out of curiosity, flips the switch, and a week later opens the app to a
 * library that is not theirs. A checkbox buried two clicks deep in a dialog
 * cannot carry that weight — the app itself has to say so, on every screen,
 * with no way to dismiss it.
 *
 * Reads `currentMock()` rather than the store's `enabled` flag on purpose.
 * `enabled` is an INTENT, settled on the next reload; this is a fact about the
 * backend actually answering right now. They disagree for exactly as long as it
 * takes to reload, and during that window the honest answer is the one the
 * running app can observe. It is also the only reading that stays correct when
 * `installMock` catches a broken fixture and falls back to the real daemon —
 * `enabled` would still be true, and the banner would be lying.
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
