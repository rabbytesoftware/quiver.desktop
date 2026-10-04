import { indicatorShownByDefault, normaliseChannel } from '../lib/build-info';
import { useBuildIndicatorStore } from '../stores/build-indicator-store';

/** This desktop's own channel, from what the release pipeline baked in; an unstamped build is `dev`. */
function buildChannel() {
	return normaliseChannel(import.meta.env.VITE_QUIVER_BUILD_CHANNEL) ?? 'dev';
}

export interface BuildIndicatorSetting {
	/** Whether the indicator is on the rail right now. */
	shown: boolean;
	/** What it would be without a choice: off on stable, on everywhere else. */
	shownByDefault: boolean;
	/** The person's choice differs from the default, so there is something to reset. */
	changed: boolean;
	set: (shown: boolean) => void;
	reset: () => void;
}

/**
 * The build indicator's on/off setting. Nothing is stored until the person
 * picks something other than the default, so a build that changes channel
 * (a nightly user moving to stable) follows its new default instead of
 * carrying an old build's behaviour along.
 */
export function useBuildIndicatorSetting(): BuildIndicatorSetting {
	const stored = useBuildIndicatorStore((s) => s.shown);
	const setStored = useBuildIndicatorStore((s) => s.setShown);
	const resetStored = useBuildIndicatorStore((s) => s.reset);

	const shownByDefault = indicatorShownByDefault(buildChannel());
	const shown = stored ?? shownByDefault;

	return {
		shown,
		shownByDefault,
		changed: shown !== shownByDefault,
		set: (next) => (next === shownByDefault ? resetStored() : setStored(next)),
		reset: resetStored,
	};
}
