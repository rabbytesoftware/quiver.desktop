use crate::connection::ConnectionManager;
use tauri::State;

#[tauri::command]
pub async fn register_arrow(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), String> {
	state.http()
		.await
		.register_arrow(&namespace)
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_arrow(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), String> {
	state.http()
		.await
		.remove_arrow(&namespace)
		.await
		.map_err(|e| e.to_string())
}
