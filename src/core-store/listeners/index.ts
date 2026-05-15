import { listen } from '@tauri-apps/api/event';

import type { ArrowListItem } from '@/domain/arrow';
import type { ConnectionStatus } from '@/domain/connection';

import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';
import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';

export async function setupListeners(): Promise<void> {
	await listen<{ status: ConnectionStatus }>('core://status', (e) => {
		useStatusStore.getState().setStatus(e.payload.status);
		if (e.payload.status === 'starting') {
			useArrowStore.getState().resetArrows();
		}
	});

	await listen<ArrowListItem[]>('arrow://hydrate', (e) => {
		useArrowStore.getState().hydrateArrows(e.payload);
	});

	await listen<{ namespace: string }>('arrow://remove', (e) => {
		useArrowStore.getState().removeArrow(e.payload.namespace);
	});

	await listen<RuntimeUpdateDTO>('runtime://update', (e) => {
		const update = toRuntimeUpdate(e.payload);
		useArrowStore.getState().applyRuntimeUpdate(update);
	});
}
