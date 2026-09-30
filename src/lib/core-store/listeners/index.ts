import type { RuntimeUpdate } from '@/domain/arrow';
import { getArrowsFor } from '@/lib/persistence/entity-cache';
import { subscribeArrowStream } from '@/lib/persistence/entity-stream';
import { maybeWipeOnVersionChange } from '@/lib/persistence/idb';
import { apiFetch, coreIsReachable } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';
import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import { announceSelf } from './self-announce';
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
		useArrowStore.getState().setCatalogRefresh(() => {});
		disposeRuntimeStream?.();
		disposeRuntimeStream = null;
	}

	function startStreams(connectionId: string): void {
		stopStreams();

		const streamGeneration = generation;

		let pendingInitialStates: RuntimeUpdate[] = [];

		const stream = subscribeArrowStream({
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
			onUnversionedUpsert: () => stream.reseed(),
		});
		disposeArrowStream = stream;
		useArrowStore.getState().setCatalogRefresh(() => stream.reseed());

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
		void announceSelf();
		await beginStreams(myGeneration).catch((err) => {
			console.error('core-store: failed to adopt an already-running core', err);
		});
		// Only once the runtime stream this generation wanted is actually
		// subscribed: see the ready branch below for why the store cannot say
		// ready any earlier than that.
		if (generation === myGeneration) {
			useStatusStore.getState().setStatus('ready');
		}
	}

	await backend().onCoreStatus(async (status) => {
		if (status === 'ready') {
			// Captured before beginStreams's own awaits, not read afterwards: it
			// must see the generation this ready belongs to, even if a later
			// `starting` (which bumps `generation`) lands before it finishes.
			// `announceSelf` is fire-and-forget on purpose -- it never blocks
			// (or is blocked by) the primary catalog load beginStreams performs.
			const myGeneration = generation;
			void announceSelf();
			await beginStreams(myGeneration);
			// The store must not claim ready before this: the runtime WS
			// subscription beginStreams just opened is the ONLY channel that
			// confirms an action's progress (see ActionButton's `pending` prop),
			// and the daemon does not replay a broadcast fired before a
			// subscriber existed. Reporting ready any earlier lets a UI gated on
			// this status act before anything is listening for the result, and
			// that action's outcome is then lost until a full reload re-seeds
			// from a REST GET. Guarded on generation for the same reason
			// beginStreams itself is: a `starting` that superseded this ready
			// while it awaited must not resurrect it as ready afterwards.
			if (generation !== myGeneration) return;
			useStatusStore.getState().setStatus('ready');
			return;
		}

		useStatusStore.getState().setStatus(status);
		if (status === 'starting') {
			generation++;
			stopStreams();
			useArrowStore.getState().reset();
		}
	});

	await adoptRunningCore();
}
