import { listen } from '@tauri-apps/api/event';

import type { ArrowListItem } from '@/domain/arrow';

import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

interface ArrowEventPayload {
	event: string;
	namespace: string;
	name?: string;
	version?: string;
}

export async function setupListeners(): Promise<void> {
	await listen<{ status: import('@/domain/connection').ConnectionStatus }>('core://status', (e) => {
		useStatusStore.getState().setStatus(e.payload.status);
		if (e.payload.status === 'starting') {
			useArrowStore.getState().resetArrows();
		}
	});

	await listen<ArrowListItem[]>('arrow://hydrate', (e) => {
		useArrowStore.getState().hydrateArrows(e.payload);
	});

	await listen<ArrowEventPayload>('arrow://event', (e) => {
		const { event, namespace, name = '', version = '' } = e.payload;
		if (event === 'removed') {
			useArrowStore.getState().removeArrow(namespace);
		} else {
			const store = useArrowStore.getState();
			const existing = store.arrows.get(namespace);
			store.upsertArrow({
				namespace,
				name,
				version,
				state: existing?.state ?? 'ready',
				active_run: existing?.active_run ?? null,
				last_outcome: existing?.last_outcome ?? null,
			});
		}
	});

	await listen<RuntimeUpdateDTO>('runtime://update', (e) => {
		const update = toRuntimeUpdate(e.payload);
		useArrowStore.getState().applyRuntimeUpdate(update);
	});
}
