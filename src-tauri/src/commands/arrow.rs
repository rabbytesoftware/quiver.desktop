use crate::connection::types::CommandError;
use crate::connection::ConnectionManager;
use tauri::State;

#[tauri::command]
pub async fn register_arrow(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), CommandError> {
	state.http()
		.await
		.register_arrow(&namespace)
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn remove_arrow(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<(), CommandError> {
	state.http()
		.await
		.remove_arrow(&namespace)
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_arrows(
	state: State<'_, ConnectionManager>,
) -> Result<serde_json::Value, CommandError> {
	state.http()
		.await
		.fetch_arrows()
		.await
		.map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_arrow_detail(
	state: State<'_, ConnectionManager>,
	namespace: String,
) -> Result<serde_json::Value, CommandError> {
	state.http()
		.await
		.get_arrow_detail(&namespace)
		.await
		.map_err(CommandError::from)
}
