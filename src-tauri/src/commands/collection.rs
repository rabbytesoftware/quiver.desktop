use crate::core_client::CoreClient;
use tauri::State;

#[tauri::command]
pub async fn follow_collection(
	state: State<'_, CoreClient>,
	namespace: String,
) -> Result<(), String> {
	state.http
		.follow_collection(&namespace)
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unfollow_collection(
	state: State<'_, CoreClient>,
	namespace: String,
) -> Result<(), String> {
	state.http
		.unfollow_collection(&namespace)
		.await
		.map_err(|e| e.to_string())
}
