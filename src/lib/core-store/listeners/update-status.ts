import { listen } from '@tauri-apps/api/event';

import type { CoreUpdateStatus } from '../store/update-status';

export async function listenForCoreUpdateStatus(callback: (status: CoreUpdateStatus) => void): Promise<() => void> {
	return listen<{ status: CoreUpdateStatus }>('core://update_status', (event) => {
		callback(event.payload.status);
	});
}
