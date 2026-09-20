import { backend } from '@/lib/transport/backend';

import type { CoreUpdateStatus } from '../store/update-status';

export async function listenForCoreUpdateStatus(callback: (status: CoreUpdateStatus) => void): Promise<() => void> {
	return backend().onCoreUpdateStatus(callback);
}
