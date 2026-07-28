pub mod commands;
pub mod connection;
pub mod fdlimit;
#[cfg(target_os = "macos")]
pub mod menu;

use connection::ConnectionManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	// Before anything opens a descriptor: this process dials quiver.core over a
	// unix socket for every frontend call and holds two long-lived event
	// streams, and macOS starts a GUI app at launchd's soft limit of 256. No
	// logger exists this early, so the outcome is reported from setup() below.
	let fd_limit = fdlimit::raise();

	// `mut` is only consumed by the cfg-gated blocks below, so a build where
	// every one of them is compiled out (a non-macOS release) leaves it
	// genuinely unused. That is the intended outcome, not a mistake — but
	// `cargo clippy -- -D warnings` would still fail the build over it.
	#[allow(unused_mut)]
	let mut builder = tauri::Builder::default()
		.plugin(tauri_plugin_log::Builder::new()
			.level(log::LevelFilter::Info)
			.build())
		.plugin(tauri_plugin_store::Builder::default().build())
		.plugin(tauri_plugin_shell::init())
		.plugin(tauri_plugin_opener::init());

	// Dev-only: exposes the webview to the Tauri MCP server (WebSocket :9223)
	// so an agent can drive and inspect the running app. Gated to debug builds
	// so a release never registers it and never opens that port. (The crate is
	// still an unconditional dependency and so still compiles; it is the
	// registration that is gated, and LTO strips what goes uncalled.)
	#[cfg(debug_assertions)]
	{
		builder = builder.plugin(tauri_plugin_mcp_bridge::init());
	}

	// Custom macOS menu that frees Cmd+Z from native capture so the webview can
	// handle it. See the menu module for what is omitted and why.
	#[cfg(target_os = "macos")]
	{
		builder = builder.menu(menu::build);
	}

	builder.manage(ConnectionManager::new())
		.setup(move |app| {
			// Report the descriptor ceiling now that a logger exists. It is the
			// first number to reach for when the app cannot dial quiver.core.
			match &fd_limit {
				fdlimit::Outcome::Failed(_) => log::warn!("{fd_limit}"),
				outcome => log::info!("{outcome}"),
			}

			let handle = app.handle().clone();
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
