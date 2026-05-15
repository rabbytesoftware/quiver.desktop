use crate::core_client::CoreClient;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn install(
	state: State<'_, CoreClient>,
	namespace: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), String> {
	state.http
		.install(&namespace, variables.unwrap_or_default())
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall(
	state: State<'_, CoreClient>,
	namespace: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), String> {
	state.http
		.uninstall(&namespace, variables.unwrap_or_default())
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute(
	state: State<'_, CoreClient>,
	namespace: String,
	method: String,
	variables: Option<HashMap<String, String>>,
) -> Result<(), String> {
	state.http
		.execute(&namespace, &method, variables.unwrap_or_default())
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(state: State<'_, CoreClient>, namespace: String) -> Result<(), String> {
	state.http.stop(&namespace).await.map_err(|e| e.to_string())
}
