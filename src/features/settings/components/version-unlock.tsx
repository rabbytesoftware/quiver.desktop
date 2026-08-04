import { useState } from 'react';

import { useMockStore } from '@/lib/mock/store';

export const UNLOCK_CLICKS = 7;

/**
 * The Developer tab's door in a release build; in dev the tab is always there.
 * Seven taps because it cannot be found by accident and can still be described
 * over a support chat in one sentence.
 */
export function VersionUnlock() {
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
				aria-label={`Quiver version ${version}`}
			>
				Quiver {version}
			</button>
			{/* Silent until most of the way there: a countdown from the first tap
			    would make it discoverable by accident. */}
			{!unlocked && clicks >= 3 && (
				<span className="ml-2 text-xs text-muted-foreground">
					{remaining} more {remaining === 1 ? 'tap' : 'taps'}…
				</span>
			)}
			{unlocked && <span className="ml-2 text-xs text-muted-foreground">Developer tab unlocked.</span>}
		</div>
	);
}
