import { useCallback, useEffect, useRef, useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowDetail, PendingActivation } from '@/domain/arrow';
import {
	classifyUpdateError,
	deriveUpdateState,
	isConnectionDrop,
	type UpdateError,
	type UpdateState,
} from '@/features/updates/lib/update-state';
import { useActivate, useArrowStore, useCheckForUpdate, useUpdate } from '@/lib/core-store';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';

const RESTART_POLL_MS = 1500;
const RESTART_GIVE_UP_MS = 60_000;

export interface ArrowUpdate {
	/** The ref this row is on now; empty until the detail has loaded. */
	installed: string;
	/** The newer ref core found ahead, or `null` when current. */
	available: string | null;
	pending: PendingActivation | null;
	state: UpdateState;
	error: UpdateError | null;
	/** A version check is in flight. Separate from `state`: it can happen in any of them. */
	checking: boolean;
	/** Each of these rejects after recording `error`, so a caller with its own error surface can show it too. */
	check(): Promise<void>;
	update(): Promise<void>;
	activate(): Promise<void>;
	dismissError(): void;
}

function stagingKey(pending: PendingActivation | null | undefined): string {
	return pending ? `${pending.version}@${pending.staged_at}` : '';
}

/**
 * The update lifecycle of one arrow over the generic runtime calls, shared by
 * the arrow page and Settings, Engine. It knows nothing about which arrow it is
 * driving: whether an update stages for a restart is the manifest's business and
 * reaches here only as `pending_activation`.
 *
 * Takes the caller's `detail` (live state already overlaid) instead of fetching
 * one, so the arrow page, which has assembled its own, does not read it twice.
 */
export function useArrowUpdate(detail: ArrowDetail | undefined): ArrowUpdate {
	const queryClient = useQueryClient();
	const checkMutation = useCheckForUpdate();
	const updateMutation = useUpdate();
	const activateMutation = useActivate();

	const [restarting, setRestarting] = useState(false);
	const [error, setError] = useState<UpdateError | null>(null);
	const [checking, setChecking] = useState(false);

	const namespace = detail?.namespace ?? '';
	const pending = detail?.pending_activation ?? null;
	const available = detail?.available?.ref ?? null;
	const running = detail?.state === 'updating' || detail?.active_run?.method === 'update';

	const reread = useCallback(
		() => queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix }),
		[queryClient]
	);

	// A runtime frame that changes what is staged is the signal to re-read; the
	// detail is the source of truth for the value itself, because nothing
	// resends a frame to a socket that reconnects after a restart.
	const livePending = useArrowStore((state) => state.arrows.get(namespace)?.pending_activation);
	const liveKey = stagingKey(livePending);
	const lastLiveKey = useRef(liveKey);
	useEffect(() => {
		if (lastLiveKey.current === liveKey) return;
		lastLiveKey.current = liveKey;
		void reread();
	}, [liveKey, reread]);

	useEffect(() => {
		if (!restarting) return;
		const poll = setInterval(() => void reread(), RESTART_POLL_MS);
		const giveUp = setTimeout(() => {
			setRestarting(false);
			setError({ kind: 'offline', message: '' });
		}, RESTART_GIVE_UP_MS);
		return () => {
			clearInterval(poll);
			clearTimeout(giveUp);
		};
	}, [restarting, reread]);

	// The new process reporting nothing staged is the only proof the handover
	// finished: a reply to `activate` or a reconnected socket is not.
	const finished = restarting && detail !== undefined && pending === null;
	useEffect(() => {
		if (!finished) return;
		setRestarting(false);
		useArrowStore.getState().refreshCatalog();
		void reread();
	}, [finished, reread]);

	async function guarded(run: () => Promise<void>): Promise<void> {
		setError(null);
		try {
			await run();
		} catch (err) {
			setError(classifyUpdateError(err));
			throw err;
		}
	}

	async function check(): Promise<void> {
		setChecking(true);
		try {
			await guarded(async () => {
				await checkMutation.mutateAsync({ namespace });
				await reread();
			});
		} finally {
			setChecking(false);
		}
	}

	function update(): Promise<void> {
		return guarded(async () => {
			const outcome = await updateMutation.mutateAsync({ namespace });
			if (outcome === 'current') await reread();
		});
	}

	async function activate(): Promise<void> {
		setError(null);
		setRestarting(true);
		try {
			const outcome = await activateMutation.mutateAsync({ namespace });
			if (outcome === 'nothing_pending') {
				setRestarting(false);
				await reread();
			}
		} catch (err) {
			if (isConnectionDrop(err)) return;
			setRestarting(false);
			setError(classifyUpdateError(err));
			throw err;
		}
	}

	return {
		installed: detail?.resolved_ref ?? '',
		available,
		pending,
		state: deriveUpdateState({
			restarting,
			running,
			error,
			pending: pending !== null,
			available: available !== null,
		}),
		error,
		checking,
		check,
		update,
		activate,
		dismissError: () => setError(null),
	};
}
