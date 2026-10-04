//! Tauri commands the shell uses on behalf of arrow pages. Arrow pages never
//! call these: they have no Tauri IPC (verified), the shell relays for them.

use tauri::ipc::Channel;
use tauri::State;

use crate::arrow_app::hosts::ArrowHosts;
use crate::arrow_app::ws::{ws_key, ws_target};
use crate::connection::bridge::{open_bridge, WsBridgeManager};
use crate::connection::ConnectionManager;

/// Registers `namespace` and returns the host its iframe must use.
#[tauri::command]
pub fn arrow_app_host(namespace: String, hosts: State<'_, ArrowHosts>) -> String {
	hosts.register(&namespace)
}

#[tauri::command]
pub async fn arrow_ws_open(
	host: String,
	conn_id: String,
	path: String,
	on_message: Channel<String>,
	hosts: State<'_, ArrowHosts>,
	manager: State<'_, WsBridgeManager>,
	connections: State<'_, ConnectionManager>,
) -> Result<(), String> {
	let target = ws_target(&hosts, &host, &path)?;
	let transport = connections.transport().await;
	open_bridge(
		transport.as_ref(),
		ws_key(&host, &conn_id),
		target,
		on_message,
		&manager,
	)
	.await
	.map(|_reader| ())
}

#[tauri::command]
pub async fn arrow_ws_send(
	host: String,
	conn_id: String,
	data: String,
	manager: State<'_, WsBridgeManager>,
) -> Result<(), String> {
	manager.send(&ws_key(&host, &conn_id), data)
}

#[tauri::command]
pub async fn arrow_ws_close(
	host: String,
	conn_id: String,
	manager: State<'_, WsBridgeManager>,
) -> Result<(), String> {
	manager.close(&ws_key(&host, &conn_id));
	Ok(())
}
