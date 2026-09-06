import { useCallback } from 'react';

import { useConnectionStore } from '@/lib/connection/store';
import {
	useAddConnection,
	useRemoveConnection,
	useRenameConnection,
	useStatusStore,
	useSwitchConnection,
} from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { useRemoteStore } from '../stores/remote-store';

export interface AddRemoteInput {
	name: string;
	url: string;
	code: string;
}

/** Every mutation this feature performs, plus the toast/dialog-close side
 *  effect each one has on success. Shared by the manage screen, the row
 *  menu, and the command palette so "switch and toast" is written once. */
export function useConnectionActions() {
	const { t } = useTranslation();
	const pushToast = useRemoteStore((s) => s.pushToast);
	const closeAdd = useRemoteStore((s) => s.closeAdd);
	const closeRename = useRemoteStore((s) => s.closeRename);
	const closeRemove = useRemoteStore((s) => s.closeRemove);

	const switchConnectionMutation = useSwitchConnection();
	const addConnectionMutation = useAddConnection();
	const renameConnectionMutation = useRenameConnection();
	const removeConnectionMutation = useRemoveConnection();

	const connect = useCallback(
		async (id: string, name: string) => {
			try {
				await switchConnectionMutation.mutateAsync({ id });
			} catch {
				pushToast(t('remote.toast.connectFailed', { name }));
				return;
			}
			// `switch_connection` does not itself emit `connection://changed` --
			// the connections LIST is unchanged by a switch, only which entry is
			// active, so that is the one thing updated here rather than waiting
			// on an event that never comes for this call.
			const current = useConnectionStore.getState();
			current.setFromEvent(current.connections, id);
			// The Rust side calls `start()` (which emits core://status) before
			// the switch command returns, so the resulting status is already
			// current by the time this promise resolves.
			const status = useStatusStore.getState().status;
			pushToast(
				status === 'disconnected'
					? t('remote.toast.disconnected', { name })
					: t('remote.toast.connected', { name })
			);
		},
		[switchConnectionMutation, pushToast, t]
	);

	const addRemote = useCallback(
		async (input: AddRemoteInput) => {
			try {
				await addConnectionMutation.mutateAsync(input);
			} catch {
				pushToast(t('remote.toast.addFailed', { name: input.name }));
				return;
			}
			closeAdd();
			pushToast(t('remote.toast.added', { name: input.name }));
		},
		[addConnectionMutation, closeAdd, pushToast, t]
	);

	const renameConnection = useCallback(
		async (id: string, name: string) => {
			try {
				await renameConnectionMutation.mutateAsync({ id, name });
			} catch {
				pushToast(t('remote.toast.renameFailed', { name }));
				return;
			}
			closeRename();
			pushToast(t('remote.toast.renamed', { name }));
		},
		[renameConnectionMutation, closeRename, pushToast, t]
	);

	const removeConnection = useCallback(
		async (id: string, name: string) => {
			try {
				await removeConnectionMutation.mutateAsync({ id });
			} catch {
				pushToast(t('remote.toast.removeFailed', { name }));
				return;
			}
			closeRemove();
			pushToast(t('remote.toast.removed', { name }));
		},
		[removeConnectionMutation, closeRemove, pushToast, t]
	);

	return {
		connect,
		addRemote,
		renameConnection,
		removeConnection,
		addBusy: addConnectionMutation.isPending,
	};
}
