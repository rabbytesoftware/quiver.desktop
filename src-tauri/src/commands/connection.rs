use serde::Serialize;
use tauri::{AppHandle, State};

use crate::connection::types::{CommandError, ConnectionConfig, Emitter};
use crate::connection::ConnectionManager;

fn conn_err(msg: String) -> CommandError {
	CommandError { code: 503, message: msg }
}

async fn emit_connection_changed(app: &AppHandle, state: &ConnectionManager) {
	let (connections, active_id) = state.get_connections(app).await;
	app.emit_connection_changed(serde_json::json!({
		"connections": connections,
		"active_id": active_id,
	}));
}

#[derive(Serialize)]
pub struct ConnectionsState {
	pub connections: Vec<ConnectionConfig>,
	pub active_id: String,
}

#[tauri::command]
pub async fn get_connections(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
) -> Result<ConnectionsState, CommandError> {
	let (connections, active_id) = state.get_connections(&app).await;
	Ok(ConnectionsState { connections, active_id })
}

#[tauri::command]
pub async fn add_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	name: String,
	url: String,
	token: String,
) -> Result<ConnectionConfig, CommandError> {
	let config = state.add_connection(&app, name, url, token).await.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(config)
}

#[tauri::command]
pub async fn remove_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), CommandError> {
	state.remove_connection(&app, &id).await.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(())
}

#[tauri::command]
pub async fn switch_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), CommandError> {
	state.switch_connection(&app, &id).await.map_err(conn_err)?;
	Ok(())
}

#[tauri::command]
pub async fn rename_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
	name: String,
) -> Result<(), CommandError> {
	state.rename_connection(&app, &id, name).await.map_err(conn_err)?;
	emit_connection_changed(&app, &state).await;
	Ok(())
}
