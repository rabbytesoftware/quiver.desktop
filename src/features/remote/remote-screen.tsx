import type { JSX } from 'react';

import { Button } from '@/components/ui/button';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { PlusIcon } from 'lucide-react';

import { AddConnectionDialog } from './components/add-connection-dialog';
import { ConnectionPanel } from './components/connection-panel';
import { RemoveConnectionDialog } from './components/remove-connection-dialog';
import { RenameConnectionDialog } from './components/rename-connection-dialog';
import { useConnectionActions } from './hooks/use-connection-actions';
import { connectionRows } from './lib/connection-rows';
import { useRemoteStore } from './stores/remote-store';

export function RemoteScreen(): JSX.Element {
	const { t } = useTranslation();
	const connections = useConnectionStore((s) => s.connections);
	const activeId = useConnectionStore((s) => s.activeId);
	const status = useStatusStore((s) => s.status);

	const addOpen = useRemoteStore((s) => s.addOpen);
	const openAdd = useRemoteStore((s) => s.openAdd);
	const closeAdd = useRemoteStore((s) => s.closeAdd);
	const renameId = useRemoteStore((s) => s.renameId);
	const openRename = useRemoteStore((s) => s.openRename);
	const closeRename = useRemoteStore((s) => s.closeRename);
	const removeId = useRemoteStore((s) => s.removeId);
	const openRemove = useRemoteStore((s) => s.openRemove);
	const closeRemove = useRemoteStore((s) => s.closeRemove);
	const openMenuId = useRemoteStore((s) => s.openMenuId);
	const toggleMenu = useRemoteStore((s) => s.toggleMenu);
	const closeMenu = useRemoteStore((s) => s.closeMenu);

	const { connect, addRemote, renameConnection, removeConnection, addBusy } = useConnectionActions();

	// The connections list arrives from a one-shot fetch plus a live event
	// subscription (`setupConnectionListeners`) at app start -- until either
	// has landed, the array is empty. Local is always present once it has;
	// there is no other legitimate "zero connections" state to confuse this with.
	const loading = connections.length === 0;
	const rows = connectionRows(connections, activeId, status);

	function nameOf(id: string | null): string {
		return connections.find((connection) => connection.id === id)?.name ?? '';
	}

	return (
		<div className="mx-auto w-full max-w-[720px] px-6 py-6">
			<div className="mb-4 flex items-end justify-between border-b border-border pb-2.5">
				<div>
					<h2 className="text-xl font-semibold tracking-[-0.3px]">{t('remote.title')}</h2>
					<p className="mt-1 text-[12.5px] text-muted-foreground">{t('remote.subtitle')}</p>
				</div>
				<Button onClick={openAdd}>
					<PlusIcon aria-hidden="true" />
					{t('remote.addButton')}
				</Button>
			</div>

			<ConnectionPanel
				loading={loading}
				onAddRemote={openAdd}
				onCloseMenu={closeMenu}
				onRemove={openRemove}
				onRename={openRename}
				onRetry={(id) => void connect(id, nameOf(id))}
				onSwitch={(id) => void connect(id, nameOf(id))}
				onSwitchToLocal={() => void connect('local', nameOf('local'))}
				onToggleMenu={toggleMenu}
				openMenuId={openMenuId}
				rows={rows}
			/>

			<AddConnectionDialog
				busy={addBusy}
				onOpenChange={(open) => !open && closeAdd()}
				onSubmit={addRemote}
				open={addOpen}
			/>

			<RenameConnectionDialog
				initialName={nameOf(renameId)}
				onOpenChange={(open) => !open && closeRename()}
				onSubmit={(name) => renameId && void renameConnection(renameId, name)}
				open={renameId !== null}
			/>

			<RemoveConnectionDialog
				isActive={removeId === activeId}
				name={nameOf(removeId)}
				onConfirm={() => removeId && void removeConnection(removeId, nameOf(removeId))}
				onOpenChange={(open) => !open && closeRemove()}
				open={removeId !== null}
			/>
		</div>
	);
}
