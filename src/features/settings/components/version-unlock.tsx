import { useState } from 'react';

import { useTranslation } from '@/lib/i18n';
import { useMockStore } from '@/lib/mock/store';

export const UNLOCK_CLICKS = 7;

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
				aria-label={t('settings.version.label', { version })}
			>
				{t('settings.version.text', { version })}
			</button>
			{!unlocked && clicks >= 3 && (
				<span className="ml-2 text-xs text-muted-foreground">
					{t('settings.version.remaining', { count: remaining })}
				</span>
			)}
			{unlocked && <span className="ml-2 text-xs text-muted-foreground">{t('settings.version.unlocked')}</span>}
		</div>
	);
}
