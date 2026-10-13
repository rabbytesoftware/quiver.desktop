import type { ConnectionConfig, ConnectionStatus } from '@/domain/connection';

export interface ConnectionRowView {
	id: string;
	name: string;
	kind: ConnectionConfig['kind'];
	subtitle: string | null;
	isLocal: boolean;
	isRemote: boolean;
	isActive: boolean;
	/** Only the active connection has a live status -- a saved-but-inactive
	 *  connection is not dialled, so there is nothing to report. */
	statusKind: ConnectionStatus | null;
	showConnect: boolean;
	/** Local can never be renamed or removed, but it can still be switched
	 *  away from -- so its menu button only shows once it is inactive. */
	showMenuBtn: boolean;
}

export function connectionRows(
	connections: ConnectionConfig[],
	activeId: string,
	status: ConnectionStatus
): ConnectionRowView[] {
	const hasRemote = connections.some((c) => c.kind !== 'local');
	const visible = hasRemote ? connections : connections.filter((c) => c.kind !== 'local');

	return visible.map((connection) => {
		const isLocal = connection.kind === 'local';
		const isActive = connection.id === activeId;
		const showConnect = !isActive;

		return {
			id: connection.id,
			name: connection.name,
			kind: connection.kind,
			subtitle: isLocal ? null : (connection.url ?? null),
			isLocal,
			isRemote: !isLocal,
			isActive,
			statusKind: isActive ? status : null,
			showConnect,
			showMenuBtn: showConnect || !isLocal,
		};
	});
}
