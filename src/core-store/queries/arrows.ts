import { useQuery } from '@tanstack/react-query';

import type { ArrowDetailDTO } from '../dtos/v0/arrow';

import { fetchArrowDetail } from '../http';
import { queryKeys } from './index';

export function useArrowDetail(namespace: string) {
	return useQuery<ArrowDetailDTO>({
		queryKey: queryKeys.arrowDetail(namespace),
		queryFn: () => fetchArrowDetail(namespace),
		staleTime: Infinity,
	});
}
