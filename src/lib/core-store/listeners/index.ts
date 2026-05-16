import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

import type { ArrowListResponseItemDTO } from '../dtos/v0/arrow';
import { toArrowListItems } from '../dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';
import { useConnectionStore } from '@/lib/connection/store';

interface ArrowEventPayload {
	event:        string;
	namespace:    string;
	name?:        string;
	description?: string;
	tags?:        string[];
	icon?:        string | null;
	banner?:      string | null;
}

interface ConnectionChangedPayload {
	connections: import('@/domain/connection').ConnectionConfig[];
	active_id:   string;
}

async function hydrateAll(): Promise<void> {
	if (import.meta.env.DEV) console.log('[core-store] hydrateAll — pulling get_arrows + get_connections');
	const [arrows, connectionState] = await Promise.all([
		invoke<ArrowListResponseItemDTO[]>('get_arrows'),
		invoke<{ connections: import('@/domain/connection').ConnectionConfig[], active_id: string }>('get_connections'),
	]);
	const items = toArrowListItems(arrows);
	if (import.meta.env.DEV) console.log(`[core-store] hydrated ${items.length} arrows, active connection: ${connectionState.active_id}`);
	for (const item of items) {
		useArrowStore.getState().upsertArrow(item);
	}
	useConnectionStore
		.getState()
		.setFromEvent(connectionState.connections, connectionState.active_id);
}

export async function setupListeners(): Promise<void> {
	await listen<{ status: import('@/domain/connection').ConnectionStatus }>('core://status', async (e) => {
		if (import.meta.env.DEV) console.log(`[core-store] core://status → ${e.payload.status}`);
		useStatusStore.getState().setStatus(e.payload.status);
		if (e.payload.status === 'starting') {
			useArrowStore.getState().resetArrows();
		}
		if (e.payload.status === 'ready') {
			await hydrateAll();
		}
	});

	await listen<ArrowEventPayload>('arrow://event', (e) => {
		if (import.meta.env.DEV) console.log(`[core-store] arrow://event → ${e.payload.event} ${e.payload.namespace}`);
		const { event, namespace, name = '', description = '', tags = [], icon = null, banner = null } = e.payload;
		if (event === 'removed') {
			useArrowStore.getState().removeArrow(namespace);
		} else {
			const store = useArrowStore.getState();
			const existing = store.arrows.get(namespace);
			store.upsertArrow({
				namespace,
				name,
				description,
				tags,
				icon,
				banner,
				version:     existing?.version ?? '',
				state:       existing?.state ?? 'ready',
				active_run:  existing?.active_run ?? null,
				last_return: existing?.last_return ?? null,
			});
		}
	});

	await listen<RuntimeUpdateDTO>('runtime://update', (e) => {
		useArrowStore.getState().applyRuntimeUpdate(toRuntimeUpdate(e.payload));
	});

	await listen<ConnectionChangedPayload>('connection://changed', (e) => {
		if (import.meta.env.DEV) console.log(`[core-store] connection://changed → active: ${e.payload.active_id}, count: ${e.payload.connections.length}`);
		useConnectionStore
			.getState()
			.setFromEvent(e.payload.connections, e.payload.active_id);
	});
}
