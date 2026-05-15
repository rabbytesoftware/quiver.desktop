pub mod commands;
pub mod connection;
pub mod core_client;

use core_client::CoreClient;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_store::Builder::default().build())
		.plugin(tauri_plugin_shell::init())
		.plugin(tauri_plugin_opener::init())
		.manage(CoreClient::new())
		.setup(|app| {
			let handle = app.handle().clone();
			let client = CoreClient::new();
			tauri::async_runtime::spawn(async move {
				client.start(handle).await;
			});
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::arrow::register_arrow,
			commands::arrow::remove_arrow,
			commands::collection::follow_collection,
			commands::collection::unfollow_collection,
			commands::runtime::install,
			commands::runtime::uninstall,
			commands::runtime::execute,
			commands::runtime::stop,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
