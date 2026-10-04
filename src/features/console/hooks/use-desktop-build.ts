import { useEffect, useMemo, useState } from 'react';

import { describeDesktop, type BuildDescriptor } from '@/features/console/lib/build-info';
import { backend, type BuildStamp } from '@/lib/transport/backend';


const UNSTAMPED: BuildStamp = { commit: null, built_at: null, label: null };

/** Whatever the native side sent, as a stamp: a missing answer or a missing field reads as unstamped. */
function readStamp(raw: Partial<BuildStamp> | null | undefined): BuildStamp {
	return {
		commit: raw?.commit ?? null,
		built_at: raw?.built_at ?? null,
		label: raw?.label ?? null,
	};
}

/**
 * What this desktop build says it is. The stamps are compile-time constants on
 * the native side, so they are read once; until they arrive this is null and the
 * indicator shows a placeholder. A shell too old to answer reads as unstamped
 * (`dev`), which is the honest description of it.
 */
export function useDesktopBuild(): BuildDescriptor | null {
	const [stamp, setStamp] = useState<BuildStamp | null>(null);

	useEffect(() => {
		let alive = true;
		backend()
			.getBuildStamp()
			.then((s) => alive && setStamp(readStamp(s)))
			.catch(() => alive && setStamp(UNSTAMPED));
		return () => {
			alive = false;
		};
	}, []);

	return useMemo(
		() =>
			stamp === null
				? null
				: describeDesktop({
						channel: import.meta.env.VITE_QUIVER_BUILD_CHANNEL?.trim() ?? '',
						label: stamp.label,
						commit: stamp.commit,
						builtAt: stamp.built_at,
					}),
		[stamp]
	);
}
