import { useQuery } from '@tanstack/react-query';

import type { Home } from '@/domain/home';
import { apiFetch, isNotFoundError } from '@/lib/transport/api';

import type { HomeDTO } from '../dtos/v0/home';
import { toHome } from '../dtos/v0/home';

export const homeQueryKey = ['home'] as const;

const POLL_MS = 3000;

const NO_HOME: Home = { shelves: [], refreshing: false };

/**
 * `GET /v0/home` -- the recommendation snapshot. Polls only while core says a
 * refresh is in flight. A core that predates the endpoint answers 404, which
 * reads as "no shelves" rather than an error.
 */
export function useHome() {
	return useQuery<Home>({
		queryKey: homeQueryKey,
		queryFn: async () => {
			try {
				return toHome(await apiFetch<HomeDTO>('/v0/home'));
			} catch (err) {
				if (isNotFoundError(err)) return NO_HOME;
				throw err;
			}
		},
		refetchInterval: (query) => (query.state.data?.refreshing ? POLL_MS : false),
	});
}
