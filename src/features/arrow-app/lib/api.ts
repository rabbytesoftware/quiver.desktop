import { Channel, invoke } from '@tauri-apps/api/core';

import type { WsApi } from './bridge';

export interface ArrowAppApi extends WsApi {
	host(namespace: string): Promise<string>;
}

/** The real implementation, over the scoped `arrow_*` Tauri commands. */
export const tauriApi: ArrowAppApi = {
	host: (namespace) => invoke<string>('arrow_app_host', { namespace }),
	async wsOpen(host, id, path, onFrame) {
		const channel = new Channel<string>();
		channel.onmessage = onFrame;
		await invoke('arrow_ws_open', { host, connId: id, path, onMessage: channel });
	},
	wsSend: (host, id, data) => invoke('arrow_ws_send', { host, connId: id, data }),
	wsClose: (host, id) => invoke('arrow_ws_close', { host, connId: id }),
};
