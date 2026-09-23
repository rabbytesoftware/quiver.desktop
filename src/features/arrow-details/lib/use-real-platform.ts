import { useEffect, useState } from 'react';

import { currentPlatform } from '@/lib/platform';
import { backend } from '@/lib/transport/backend';

/**
 * This machine's `"os/arch"` platform key, starting from the zero-latency
 * `currentPlatform()` UA guess and swapping in the native side's real answer
 * (`Backend.getPlatform()`) once it resolves.
 *
 * Any COMPATIBILITY decision -- the not-supported indicator, the add-time
 * warning -- has to use the value this returns, not the raw guess: the guess
 * can misdetect an Apple Silicon Mac's architecture (see `platform.ts`'s own
 * `currentArch` doc comment), which would false-positive a warning for
 * exactly the audience it matters most to get right for. The sync guess is
 * kept as the initial value only so the details page doesn't flash a loading
 * state on first paint -- `targetForPlatform`'s own first-target fallback
 * already tolerates a brief wrong guess gracefully, same as it always has.
 */
export function useRealPlatform(): string {
	const [platform, setPlatform] = useState<string>(currentPlatform);

	useEffect(() => {
		let cancelled = false;
		backend()
			.getPlatform()
			.then((real) => {
				if (!cancelled) setPlatform(real);
			})
			.catch(() => {});
		return () => {
			cancelled = true;
		};
	}, []);

	return platform;
}
