import { useEffect, useState } from 'react';

import type { ArrowChannel } from '@/domain/arrow';
import { QUIVER_CORE_NAMESPACE } from '@/domain/release';
import { toArrowChannels, type ChannelListDTO } from '@/lib/core-store/dtos/v0/arrow';
import { apiFetch } from '@/lib/transport/api';

interface CoreChannelsState {
	channels: ArrowChannel[];
	loading: boolean;
	/** Set when the fetch itself failed -- most commonly, quiver.core hasn't finished self-registering on a fresh daemon. */
	failed: boolean;
}

const INITIAL: CoreChannelsState = { channels: [], loading: true, failed: false };

/**
 * quiver.core's own release channels, for the self-update-channel row in
 * Settings -- a plain `GET /v0/arrow/:ns/channels` against quiver.core's own
 * self-catalogued namespace, same shape as any other arrow's channel list.
 * Kept separate from `DaemonConfig`: this is catalog data, not a config
 * section, so it needs its own fetch rather than riding along with `GET /v0/config`.
 */
export function useCoreChannels(): CoreChannelsState {
	const [state, setState] = useState<CoreChannelsState>(INITIAL);

	useEffect(() => {
		let cancelled = false;
		setState(INITIAL);
		apiFetch<ChannelListDTO>(`/v0/arrow/${encodeURIComponent(QUIVER_CORE_NAMESPACE)}/channels`)
			.then((dto) => {
				if (!cancelled) setState({ channels: toArrowChannels(dto), loading: false, failed: false });
			})
			.catch(() => {
				if (!cancelled) setState({ channels: [], loading: false, failed: true });
			});
		return () => {
			cancelled = true;
		};
	}, []);

	return state;
}
