import { listen } from '@tauri-apps/api/event';
import { useArrowStore } from './store';
import type { ArrowListItem } from '@/domain/arrow';
import type { CoreStatus } from './store';

export async function setupListeners(): Promise<void> {
    await listen<{ status: CoreStatus }>('core://status', (e) => {
        useArrowStore.getState().setStatus(e.payload.status);
    });

    await listen<ArrowListItem[]>('arrow://hydrate', (e) => {
        useArrowStore.getState().hydrateArrows(e.payload);
    });

    await listen<{ namespace: string }>('arrow://remove', (e) => {
        useArrowStore.getState().removeArrow(e.payload.namespace);
    });

    await listen<{
        namespace: string;
        state: ArrowListItem['state'];
        active_run: ArrowListItem['active_run'];
        last_outcome: ArrowListItem['last_outcome'];
    }>('runtime://update', (e) => {
        useArrowStore.getState().applyRuntimeUpdate(e.payload);
    });
}
