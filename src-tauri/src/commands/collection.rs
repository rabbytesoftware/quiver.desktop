use crate::connection::types::CommandError;
use crate::connection::ConnectionManager;
use tauri::State;

#[tauri::command]
pub async fn follow_collection(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), CommandError> {
	state.http()
		.await
		.follow_collection(&namespace)
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn unfollow_collection(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), CommandError> {
	state.http()
		.await
		.unfollow_collection(&namespace)
		.await
		.map_err(CommandError::from)
}
