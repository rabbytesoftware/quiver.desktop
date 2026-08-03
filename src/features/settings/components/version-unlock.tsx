import { useState } from 'react';

import { useMockStore } from '@/lib/mock/store';

/** How many clicks it takes. Enough to be intentional, few enough to describe over chat. */
export const UNLOCK_CLICKS = 7;

/**
 * The Developer tab's door in a release build.
 *
 * In dev the tab is simply always there. Shipped, it is hidden until this is
 * clicked seven times — the Android pattern, chosen because it has two
 * properties nothing else does: it cannot be found by accident, and it can be
 * described to someone over a support chat in one sentence.
 *
 * A keyboard chord would fail the second test (it collides with something on
 * one of three platforms), and a hidden URL would fail the first (a router
 * with a `/developer` route is one autocomplete away).
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
		<div className="mt-8 border-t border-line pt-3">
			<button
				type="button"
				onClick={tap}
				className="select-none text-[12px] text-ink-3 hover:text-ink-2"
				aria-label={`Quiver version ${version}`}
			>
				Quiver {version}
			</button>
			{/* Silent until you are most of the way there. Announcing the countdown
			    from the first click would make it discoverable by accident, which is
			    the one thing this is for. */}
			{!unlocked && clicks >= 3 && (
				<span className="ml-2 text-[12px] text-ink-3">
					{remaining} more {remaining === 1 ? 'tap' : 'taps'}…
				</span>
			)}
			{unlocked && <span className="ml-2 text-[12px] text-ink-3">Developer tab unlocked.</span>}
		</div>
	);
}
