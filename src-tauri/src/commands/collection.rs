use crate::connection::ConnectionManager;
use tauri::State;

#[tauri::command]
pub async fn follow_collection(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), String> {
	state.http()
		.await
		.follow_collection(&namespace)
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unfollow_collection(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), String> {
	state.http()
		.await
		.unfollow_collection(&namespace)
		.await
		.map_err(|e| e.to_string())
}
