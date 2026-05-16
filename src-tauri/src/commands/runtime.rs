use crate::connection::types::CommandError;
use crate::connection::ConnectionManager;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn install(
	state: State<'_, ConnectionManager>,
	namespace: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), CommandError> {
	state.http()
		.await
		.install(&namespace, variables.unwrap_or_default())
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn uninstall(
	state: State<'_, ConnectionManager>,
	namespace: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), CommandError> {
	state.http()
		.await
		.uninstall(&namespace, variables.unwrap_or_default())
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn execute(
	state: State<'_, ConnectionManager>,
	namespace: String,
	method: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), CommandError> {
	state.http()
		.await
		.execute(&namespace, &method, variables.unwrap_or_default())
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn stop(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), CommandError> {
	state.http()
		.await
		.stop(&namespace)
		.await
		.map_err(CommandError::from)
}
