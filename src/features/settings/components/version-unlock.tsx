import { useState } from 'react';

import { useTranslation } from '@/lib/i18n';
import { useMockStore } from '@/lib/mock/store';

export const UNLOCK_CLICKS = 7;

/**
 * The Developer tab's door in a release build; in dev the tab is always there.
 * Seven taps because it cannot be found by accident and can still be described
 * over a support chat in one sentence.
 */
export function VersionUnlock() {
	const { t } = useTranslation();
	const unlocked = useMockStore((s) => s.devUnlocked);
	const unlock = useMockStore((s) => s.unlockDeveloper);
	const [clicks, setClicks] = useState(0);

	const version = import.meta.env.VITE_APP_VERSION ?? '0.1.0';
	const remaining = UNLOCK_CLICKS - clicks;

	function tap() {
		if (unlocked) return;
		const next = clicks + 1;
		setClicks(next);
		if (next >= UNLOCK_CLICKS) unlock();
	}

	return (
		<div className="mt-8 border-t border-border pt-3">
			<button
				type="button"
				onClick={tap}
				className="select-none text-xs text-muted-foreground hover:text-foreground"
				// A STRING, so the interpolator leaves it alone. `0.1.0` through
				// `formatNumber` is not a number and `10` in a version would come
				// back grouped.
				aria-label={t('settings.version.label', { version })}
			>
				{t('settings.version.text', { version })}
			</button>
			{/* Silent until most of the way there: a countdown from the first tap
			    would make it discoverable by accident. */}
			{!unlocked && clicks >= 3 && (
				// The count picks the grammatical form. What was a `remaining === 1`
				// ternary here is right in English and wrong the moment a language
				// with a third plural category is added — Russian needs a different
				// word at 2 and again at 5.
				<span className="ml-2 text-xs text-muted-foreground">
					{t('settings.version.remaining', { count: remaining })}
				</span>
			)}
			{unlocked && <span className="ml-2 text-xs text-muted-foreground">{t('settings.version.unlocked')}</span>}
		</div>
	);
}
