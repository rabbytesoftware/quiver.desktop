pub mod commands;
pub mod connection;
pub mod fdlimit;
#[cfg(target_os = "macos")]
pub mod menu;

/// Serialises every test in this crate that opens a socket.
///
/// `connection::bridge`'s descriptor-leak regression test counts this process's
/// open file descriptors, which is process-wide state: a listener stood up by
/// any other test lands in the same count — in its baseline as well as in its
/// final read. That is not hypothetical. With the guard scoped to `bridge.rs`
/// alone, a deliberately reintroduced 19-descriptor leak went UNDETECTED in 4
/// of 20 parallel runs, because the baseline was taken while a dozen of the
/// transport tests' sockets were open and the leak fitted inside that noise.
/// So every test that opens a socket takes this, exactly as Crowbar's
/// `fd_tests()` does.
///
/// It lives in the crate root because that is the one module every test module
/// can name. `tokio::sync::Mutex` because the holders are all `async`, and
/// because it cannot be poisoned by a test that fails while holding it.
#[cfg(test)]
pub(crate) static FD_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use connection::bridge::{open_bridge, WsBridgeManager};
use connection::proxy::proxy_once;
use connection::ConnectionManager;
use tauri::http::Request;
use tauri::ipc::Channel;
use tauri::{Manager, Runtime, State, UriSchemeContext, UriSchemeResponder};

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

/// Open a WebSocket to whatever connection is active for `path` (a full
/// `/v0/...` route) and stream its frames to `on_message`.
///
/// The Tauri-shaped wrapper around `bridge::open_bridge`: it resolves the
/// transport PER OPEN, so a stream opened after a connection switch dials the
/// new peer with nothing to re-register. Excluded from coverage along with the
/// rest of this file — everything worth asserting lives in `bridge.rs`.
#[tauri::command]
async fn ws_open(
	conn_id: String,
	path: String,
	on_message: Channel<String>,
	manager: State<'_, WsBridgeManager>,
	connections: State<'_, ConnectionManager>,
) -> Result<(), String> {
	let transport = connections.transport().await;
	open_bridge(transport.as_ref(), conn_id, path, on_message, &manager)
		.await
		.map(|_reader| ())
}

/// Send a raw text frame (the JSON the frontend wants to publish) to the daemon.
#[tauri::command]
async fn ws_send(
	conn_id: String,
	data: String,
	manager: State<'_, WsBridgeManager>,
) -> Result<(), String> {
	manager.send(&conn_id, data)
}

/// Close the WebSocket leg for a connection.
#[tauri::command]
async fn ws_close(conn_id: String, manager: State<'_, WsBridgeManager>) -> Result<(), String> {
	manager.close(&conn_id);
	Ok(())
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
		.manage(WsBridgeManager::new())
		// A page load orphans every bridged connection the outgoing page owned:
		// its JS is gone and will never close ids it no longer remembers, and
		// the new page opens its own. Nothing else can notice — a `Channel`
		// keeps working across a reload, because a reloaded page is the same
		// webview — so if we do not retire them here, their reader tasks park
		// on sockets nobody will ever read again and hold a descriptor apiece
		// for the life of the app. See `connection::bridge`'s module doc,
		// teardown path (3).
		.on_page_load(|webview, payload| {
			if payload.event() != tauri::webview::PageLoadEvent::Started {
				return;
			}
			webview.app_handle().state::<WsBridgeManager>().close_all();
		})
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
			ws_open,
			ws_send,
			ws_close,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
