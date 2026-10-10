import type { JSX } from 'react';

import { ArrowDetailsScreen } from '@/features/arrow-details/arrow-details-screen';
import { useArrowStore } from '@/lib/core-store';
import { useArrowDetail } from '@/lib/core-store/queries/arrow';

import { ArrowAppView } from './arrow-app-view';

interface ArrowPageProps {
	/** The route's identity: bare when the link came from Search, `ns@ref` from the library. */
	namespace: string;
	onIdentityChange: (namespace: string) => void;
}

/**
 * Chooses between the arrow's running interface and its details page.
 *
 * The live store and every runtime frame are keyed by the resolved `ns@ref`,
 * which a bare route does not carry. The page resolves that identity the same
 * way the details screen does (`data.namespace`, falling back to the route's
 * own until it loads) and uses it for both the lookup and the view, so the
 * interface shows whichever way the arrow was reached. The route itself is
 * left alone: nothing here navigates, so there is no identity to flip-flop.
 */
export function ArrowPage({ namespace, onIdentityChange }: ArrowPageProps): JSX.Element {
	const { data } = useArrowDetail(namespace);
	const liveKey = data?.namespace ?? namespace;
	const hasSurface = useArrowStore((s) => Boolean(s.arrows.get(liveKey)?.active_run?.surface));

	if (hasSurface) return <ArrowAppView namespace={liveKey} onIdentityChange={onIdentityChange} />;
	return <ArrowDetailsScreen namespace={namespace} onIdentityChange={onIdentityChange} />;
}
