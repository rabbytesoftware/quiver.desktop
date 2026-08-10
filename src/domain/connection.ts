export type ConnectionKind = 'local' | 'remote';

export interface ConnectionConfig {
	id: string;
	name: string;
	kind: ConnectionKind;
	url?: string;
	api_version: string;
}

export type ConnectionStatus = 'starting' | 'ready' | 'disconnected';

export const LOCAL_CONNECTION_ID = 'local';

export function localConnection(): ConnectionConfig {
	return {
		id: LOCAL_CONNECTION_ID,
		name: 'Local',
		kind: 'local',
		api_version: 'v0',
	};
}
