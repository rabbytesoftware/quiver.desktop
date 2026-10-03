import { useMemo } from 'react';

import type { ArrowDetail } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import { useArrowDetail } from '@/lib/core-store/queries/arrow';

/**
 * One arrow's detail with the live runtime state laid over it, for surfaces
 * that need no more than that. The arrow page assembles a fuller one
 * (`useAssembledArrowDetail`); this skips the readme and dependency reads it
 * does not use. An empty `namespace` fetches nothing.
 */
export function useLiveArrowDetail(namespace: string): ArrowDetail | undefined {
	const { data } = useArrowDetail(namespace);
	const live = useArrowStore((state) => state.arrows.get(data?.namespace ?? namespace));

	return useMemo(() => {
		if (!data || !live) return data;
		return { ...data, state: live.state, active_run: live.active_run };
	}, [data, live]);
}
