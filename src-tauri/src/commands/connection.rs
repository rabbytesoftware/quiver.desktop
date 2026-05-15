use crate::connection::{ConnectionManager, types::ConnectionConfig};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn list_connections(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
) -> Result<Vec<ConnectionConfig>, String> {
	Ok(state.list_connections(&app).await)
}

#[tauri::command]
pub async fn add_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	name: String,
	url: String,
	token: String,
) -> Result<ConnectionConfig, String> {
	state.add_connection(&app, name, url, token).await
}

#[tauri::command]
pub async fn remove_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), String> {
	state.remove_connection(&app, &id).await
}

#[tauri::command]
pub async fn switch_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
) -> Result<(), String> {
	state.switch_connection(&app, &id).await
}

#[tauri::command]
pub async fn rename_connection(
	app: AppHandle,
	state: State<'_, ConnectionManager>,
	id: String,
	name: String,
) -> Result<(), String> {
	state.rename_connection(&app, &id, name).await
}
