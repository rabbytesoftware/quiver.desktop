import type { RuntimeUpdate } from '@/domain/arrow';
import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch, coreIsReachable } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';
import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import type { ArrowListResponseItemDTO } from '../dtos/v0/arrow';
import { toArrowCatalogRecords, toInitialRuntimeUpdates } from '../dtos/v0/arrow';
import type { RuntimeUpdateDTO } from '../dtos/v0/runtime';
import { toRuntimeUpdate } from '../dtos/v0/runtime';
import { useArrowStore } from '../store/arrows';
import { useStatusStore } from '../store/status';

const RUNTIME_ENDPOINT = '/v0/runtime';

export async function setupListeners(): Promise<void> {
	const wipeDone = maybeWipeOnVersionChange();

	let disposeArrowStream: (() => void) | null = null;
	let disposeRuntimeStream: (() => void) | null = null;
	let startPending = 0;
	let generation = 0;

	function stopStreams(): void {
		disposeArrowStream?.();
		disposeArrowStream = null;
		disposeRuntimeStream?.();
		disposeRuntimeStream = null;
	}

	function startStreams(connectionId: string): void {
		stopStreams();

		const streamGeneration = generation;

		let pendingInitialStates: RuntimeUpdate[] = [];

		disposeArrowStream = subscribeArrowStream({
			connectionId,
			seed: () =>
				apiFetch<ArrowListResponseItemDTO[]>('/v0/arrow?user_installed=true').then((items) => {
					pendingInitialStates = toInitialRuntimeUpdates(items);
					return toArrowCatalogRecords(items, connectionId);
				}),
			onChange: () => {
				const myBatch = pendingInitialStates;
				return getArrowsFor(connectionId).then((records) => {
					if (generation !== streamGeneration) return;
					useArrowStore.getState().setCatalog(records);
					if (pendingInitialStates !== myBatch) return;
					if (myBatch.length === 0) return;
					const visible = new Set(records.map((r) => r.namespace));
					for (const update of myBatch) {
						if (visible.has(update.namespace)) useArrowStore.getState().seedInitialState(update);
					}
					pendingInitialStates = [];
				});
			},
			onSeedError: () => {
				if (generation !== streamGeneration) return;
				useArrowStore.getState().setCatalogError();
			},
		});

		disposeRuntimeStream = wsManager.subscribe(RUNTIME_ENDPOINT, (data) => {
			if (isReconnectSentinel(data)) return;
			useArrowStore.getState().applyRuntimeUpdate(toRuntimeUpdate(data as RuntimeUpdateDTO));
		});
	}

	async function beginStreams(myGeneration: number): Promise<void> {
		startPending++;
		try {
			await wipeDone;
			const { active_id } = await backend().getConnections();
			if (generation !== myGeneration) return;
			startStreams(active_id);
		} finally {
			startPending--;
		}
	}

	async function adoptRunningCore(): Promise<void> {
		const myGeneration = generation;
		if (!(await coreIsReachable())) return;
		if (generation !== myGeneration || startPending > 0 || disposeArrowStream) return;
		useStatusStore.getState().setStatus('ready');
		await beginStreams(myGeneration).catch((err) => {
			console.error('core-store: failed to adopt an already-running core', err);
		});
	}

	await backend().onCoreStatus(async (status) => {
		useStatusStore.getState().setStatus(status);
		if (status === 'starting') {
			generation++;
			stopStreams();
			useArrowStore.getState().reset();
		}
		if (status === 'ready') {
			await beginStreams(generation);
		}
	});

	await adoptRunningCore();
}
