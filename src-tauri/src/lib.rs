pub mod commands;
pub mod connection;

use connection::ConnectionManager;
use tauri::Manager;
use tauri_plugin_decorum::WebviewWindowExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_decorum::init())
		.plugin(tauri_plugin_log::Builder::new()
			.level(log::LevelFilter::Info)
			.build())
		.plugin(tauri_plugin_store::Builder::default().build())
		.plugin(tauri_plugin_shell::init())
		.plugin(tauri_plugin_opener::init())
		.manage(ConnectionManager::new())
		.setup(|app| {
			let handle = app.handle().clone();

			if let Some(window) = app.get_webview_window("main") {
				window.create_overlay_titlebar().unwrap();

				#[cfg(target_os = "macos")]
				{
					use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
					if let Err(e) = apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, Some(10.0)) {
						log::warn!("vibrancy failed: {e}");
					}
					// x=12 aligns with standard macOS inset; y=16 centers in the 38px drag bar
					window.set_traffic_lights_inset(12.0, 16.0).unwrap();
				}

				#[cfg(target_os = "windows")]
				{
					use window_vibrancy::apply_acrylic;
					if let Err(e) = apply_acrylic(&window, Some((18, 18, 18, 125))) {
						log::warn!("acrylic failed: {e}");
					}
				}
			}

			tauri::async_runtime::spawn(async move {
				handle.state::<ConnectionManager>()
					.start(handle.clone())
					.await;
			});
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::arrow::register_arrow,
			commands::arrow::remove_arrow,
			commands::arrow::get_arrows,
			commands::arrow::get_arrow_detail,
			commands::collection::follow_collection,
			commands::collection::unfollow_collection,
			commands::runtime::install,
			commands::runtime::uninstall,
			commands::runtime::execute,
			commands::runtime::stop,
			commands::connection::get_connections,
			commands::connection::add_connection,
			commands::connection::remove_connection,
			commands::connection::switch_connection,
			commands::connection::rename_connection,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
