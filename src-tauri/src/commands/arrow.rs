use tauri::State;
use crate::core_client::CoreClient;

#[tauri::command]
pub async fn register_arrow(
    state: State<'_, CoreClient>,
    namespace: String,
) -> Result<(), String> {
    state.http.register_arrow(&namespace).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_arrow(
    state: State<'_, CoreClient>,
    namespace: String,
) -> Result<(), String> {
    state.http.remove_arrow(&namespace).await.map_err(|e| e.to_string())
}
