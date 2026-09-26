import { useEffect, useState } from 'react';

import { currentPlatform } from '@/lib/platform';
import { backend } from '@/lib/transport/backend';

export interface RealPlatform {
	/**
	 * This machine's `"os/arch"` platform key, starting from the zero-latency
	 * `currentPlatform()` UA guess and swapping in the native side's real
	 * answer (`Backend.getPlatform()`) once it resolves.
	 */
	platform: string;
	/** True once `platform` is the native side's real answer, not the UA guess. */
	resolved: boolean;
}

/**
 * Reactive display value for a platform-compatibility decision.
 *
 * `resolved` exists because the guess itself can be WRONG -- it can misdetect
 * an Apple Silicon Mac's architecture (see `platform.ts`'s own `currentArch`
 * doc comment) -- so a caller rendering a not-supported indicator off `platform`
 * before `resolved` is true risks a flicker at best and a false negative at
 * worst (briefly showing "supported" for an arrow that is not, or the reverse).
 * The sync guess is kept as the initial value only so the details page doesn't
 * flash a loading state on first paint; `targetForPlatform`'s own
 * first-target fallback already tolerates a brief wrong guess gracefully for
 * everything that isn't a hard compatibility gate.
 *
 * A one-off decision that must not be stale even by the width of a single
 * render (an add-to-library click) should use `resolveRealPlatform` instead
 * of this hook's `platform` value -- see its own doc comment for why.
 */
export function useRealPlatform(): RealPlatform {
	const [platform, setPlatform] = useState<string>(currentPlatform);
	const [resolved, setResolved] = useState(false);

	useEffect(() => {
		let cancelled = false;
		backend()
			.getPlatform()
			.then((real) => {
				if (!cancelled) {
					setPlatform(real);
					setResolved(true);
				}
			})
			.catch(() => {});
		return () => {
			cancelled = true;
		};
	}, []);

	return { platform, resolved };
}

/**
 * A fresh, authoritative read of this machine's real platform, for a
 * decision that cannot tolerate `useRealPlatform`'s own brief guess-then-
 * resolve window -- concretely, the add-to-library warning gate: gating that
 * click on the REACTIVE `platform` value risks reading the stale UA guess if
 * the click lands before `useRealPlatform`'s effect has resolved, which would
 * silently skip the warning for a genuinely unsupported arrow instead of
 * merely mis-rendering a badge. Falls back to the UA guess only if the
 * native call itself fails, matching `useRealPlatform`'s own failure mode.
 */
export async function resolveRealPlatform(): Promise<string> {
	try {
		return await backend().getPlatform();
	} catch {
		return currentPlatform();
	}
}
