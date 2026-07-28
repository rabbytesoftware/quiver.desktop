pub mod commands;
pub mod connection;
pub mod fdlimit;
#[cfg(target_os = "macos")]
pub mod menu;

use connection::proxy::proxy_once;
use connection::ConnectionManager;
use tauri::http::Request;
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

// Injected at document-start on EVERY page load, before any frontend JS runs.
// A reload wipes `window.__QUIVER__`, and without it the frontend falls back
// to the dev origin and dials a doomed ws://localhost:5173, which reads as
// "backend unavailable". Hostname-guarded so it never leaks into a webview
// showing external content.
const QUIVER_BOOTSTRAP: &str = r#"
(function () {
  var h = location.hostname;
  if (h === 'localhost' || h === 'tauri.localhost') {
    window.__QUIVER__ = Object.assign(window.__QUIVER__ || {}, {
      mode: 'local',
      api: 'quiver://localhost',
    });
  }
})();
"#;

/// `tauri::plugin::Builder::build()` returns `TauriPlugin<R, C>`, and `C` has
/// no other usage site to pin it down — inline at the call site, `.plugin()`
/// accepts any `C: DeserializeOwned`, so the compiler cannot choose one and
/// rejects the expression outright. Naming the return type here (which
/// defaults `C` to `()`, same as every other plugin in this file) is what
/// resolves it.
fn quiver_bootstrap_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("quiver-bootstrap")
		.js_init_script(QUIVER_BOOTSTRAP.to_string())
		.build()
}

/// The Tauri-shaped wrapper around `proxy::proxy_once`. Excluded from
/// coverage: it needs a live `UriSchemeResponder` and an `AppHandle`, neither
/// of which exists under test. Keep it thin — every line added here is a line
/// nothing can cover.
fn handle_request<R: Runtime>(
	ctx: UriSchemeContext<'_, R>,
	request: Request<Vec<u8>>,
	responder: UriSchemeResponder,
) {
	let app = ctx.app_handle().clone();
	tauri::async_runtime::spawn(async move {
		let transport = app.state::<ConnectionManager>().transport().await;
		let resp = proxy_once(transport.as_ref(), request).await;
		// Respond on the main thread: WKURLSchemeTask cancellation
		// (webView:stopURLSchemeTask:) is delivered there, so responding there
		// serialises with it. From a tokio worker it races cancellation, and a
		// stopped task makes WebKit throw an NSException that cannot unwind
		// through the ObjC bridge -> abort(). A process crash, not an error.
		let _ = app.run_on_main_thread(move || responder.respond(resp));
	});
}

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
		.plugin(tauri_plugin_opener::init())
		.register_asynchronous_uri_scheme_protocol("quiver", handle_request)
		.plugin(quiver_bootstrap_plugin());

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
			commands::connection::get_connections,
			commands::connection::add_connection,
			commands::connection::remove_connection,
			commands::connection::switch_connection,
			commands::connection::rename_connection,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
