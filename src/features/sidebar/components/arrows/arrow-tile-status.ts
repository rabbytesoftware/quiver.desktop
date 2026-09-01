import type { ArrowEntry } from '@/domain/arrow';
import type { ArrowIconKind, ArrowStatus } from '@/features/arrow-details/lib/status';
import { computeStatus } from '@/features/arrow-details/lib/status';

/** The only kinds worth an ArrowTile badge -- the steady `ready`/`idle`/`archived` cases stay silent. See docs/home-page-spec.md §2. */
const BADGE_KINDS: ReadonlySet<ArrowIconKind> = new Set(['busy', 'active', 'up', 'problem']);

/**
 * Every arrow reactive `useArrowStore` holds is `user_installed: true` by
 * construction (the WS listener only ever seeds library arrows) -- safe to
 * assume here rather than threading that flag through every call site.
 */
export function arrowTileStatus(entry: Pick<ArrowEntry, 'state'>): ArrowStatus | null {
	const status = computeStatus({ state: entry.state, user_installed: true });
	return BADGE_KINDS.has(status.iconKind) ? status : null;
}
