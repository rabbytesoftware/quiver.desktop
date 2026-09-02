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

/// The origin the frontend must dial to reach the `quiver://` scheme handler,
/// on Windows.
///
/// WebView2 cannot register a non-standard URI scheme at all, so wry does not
/// try: it registers `http://quiver.localhost` and rewrites every request back
/// before handing it to the handler (`wry`'s `custom_protocol_workaround.rs`,
/// `apply_uri_work_around`, turns `{scheme}://localhost/x` into
/// `{http|https}://{scheme}.localhost/x`). A frontend that fetched
/// `quiver://localhost/...` there would reach nothing — the whole data layer is
/// dead — and no amount of `cargo check --target x86_64-pc-windows-msvc` can
/// see it, because it is a string.
///
/// So the base is computed HERE, per platform, and injected. The frontend takes
/// what it is given (`window.__QUIVER__.api`) and never has to know.
#[cfg(windows)]
const QUIVER_API_BASE: &str = "http://quiver.localhost";

/// The same origin everywhere else. WKWebView (macOS/iOS) and WebKitGTK
/// (Linux) register the scheme for real, so the custom scheme is reachable
/// as written and wry applies no rewrite.
#[cfg(not(windows))]
const QUIVER_API_BASE: &str = "quiver://localhost";

/// Injected at document-start on EVERY page load, before any frontend JS runs.
///
/// A reload wipes `window.__QUIVER__`, and without it the frontend falls back
/// to the dev origin and dials a doomed ws://localhost:5173, which reads as
/// "backend unavailable". Hostname-guarded so it never leaks into a webview
/// showing external content — `quiver.localhost` is in the guard because that
/// is the hostname the Windows work-around above produces.
///
/// A `format!` rather than a `const`: the API base differs per platform (see
/// [`QUIVER_API_BASE`]), and a hardcoded literal here is exactly the defect
/// this shape exists to prevent.
fn quiver_bootstrap_script() -> String {
	format!(r#"
(function () {{
  var h = location.hostname;
  if (h === 'localhost' || h === 'tauri.localhost' || h === 'quiver.localhost') {{
    window.__QUIVER__ = Object.assign(window.__QUIVER__ || {{}}, {{
      mode: 'local',
      api: '{QUIVER_API_BASE}',
    }});
  }}
}})();
"#)
}

/// `tauri::plugin::Builder::build()` returns `TauriPlugin<R, C>`, and `C` has
/// no other usage site to pin it down — inline at the call site, `.plugin()`
/// accepts any `C: DeserializeOwned`, so the compiler cannot choose one and
/// rejects the expression outright. Naming the return type here (which
/// defaults `C` to `()`, same as every other plugin in this file) is what
/// resolves it.
fn quiver_bootstrap_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("quiver-bootstrap")
		.js_init_script(quiver_bootstrap_script())
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
		// Handed to `proxy_once` UNRESOLVED, so the lookup happens inside its
		// timeout. `.transport()` takes `ConnectionManager`'s read lock, and
		// tokio's `RwLock` is fair: a reader arriving while a writer waits
		// queues behind that writer. Awaited here instead, that wait is
		// unbounded — no timeout in this app can reach it.
		let resolve = {
			let app = app.clone();
			async move { app.state::<ConnectionManager>().transport().await }
		};
		let resp = proxy_once(resolve, request).await;
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
			commands::connection::check_remote_health,
			commands::connection::add_connection,
			commands::connection::remove_connection,
			commands::connection::switch_connection,
			commands::connection::rename_connection,
			ws_open,
			ws_send,
			ws_close,
		])
		// `build` + `run(callback)` rather than `run(context)`, for the sake of the
		// callback: it is the only place the app can notice that it is exiting, and
		// quitting used to leave the local daemon running. `CommandChild` has no
		// `Drop`, so nothing in the process's own teardown reaches it — see
		// `SidecarManager::reap`.
		.build(tauri::generate_context!())
		.expect("error while building tauri application")
		.run(|app, event| {
			// Both, not one: `ExitRequested` is the last chance before a normal
			// quit, and `Exit` also covers `app.exit()` and a quit nothing asked
			// for. Both fire on the usual path, so what they reach is idempotent
			// (`ConnectionManager::shutdown`).
			if matches!(
				event,
				tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
			) {
				// `block_on`, because the callback is synchronous and the process is
				// about to go: a task spawned here would be dropped with the runtime
				// before it reached the kill. Safe from this thread — the event loop
				// is not a tokio worker — and bounded, because `shutdown` takes a
				// read lock nothing holds across an await and ends in one `kill()`.
				tauri::async_runtime::block_on(
					app.state::<ConnectionManager>().shutdown(),
				);
			}
		});
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The one thing in this file that a compiler cannot check: the API base is
	/// a string, and a wrong one leaves the entire data layer dead on the
	/// platform it is wrong for, silently. So the expectation is written out
	/// per platform, in literals, rather than derived from the constant — a
	/// test that says `QUIVER_API_BASE == QUIVER_API_BASE` guards nothing.
	///
	/// Breaking either side fails a build somewhere: the non-Windows literal
	/// fails here on every developer machine and on macOS/Linux CI, the Windows
	/// literal fails on a Windows runner.
	#[test]
	fn the_injected_api_base_matches_the_platform() {
		let expected = if cfg!(windows) {
			// WebView2 cannot register a custom scheme, so wry rewrites
			// `quiver://localhost/x` to `http://quiver.localhost/x`. Dialling
			// the un-rewritten form there reaches nothing at all.
			"http://quiver.localhost"
		} else {
			"quiver://localhost"
		};
		assert_eq!(QUIVER_API_BASE, expected);
	}

	/// The constant is only useful if it actually reaches the page. The
	/// frontend reads `window.__QUIVER__.api` and nothing else, so the script
	/// has to carry the platform's base verbatim.
	#[test]
	fn the_bootstrap_script_injects_the_platform_api_base() {
		let script = quiver_bootstrap_script();
		assert!(
			script.contains(&format!("api: '{QUIVER_API_BASE}'")),
			"the bootstrap must inject the computed base; got {script}"
		);
		// And not the other platform's. This is what catches a base hardcoded
		// back into the script: the wrong literal is invisible on the platform
		// it happens to match, and fails loudly on the one it does not.
		let wrong = if cfg!(windows) {
			"quiver://localhost"
		} else {
			"http://quiver.localhost"
		};
		assert!(
			!script.contains(wrong),
			"the bootstrap must not carry another platform's API base ({wrong}); \
			 got {script}"
		);
	}

	/// The shell plugin is registered above, but for RUST's use only: the sidecar
	/// is spawned through `app.shell().sidecar(...)` and killed through the
	/// `CommandChild` that spawn returns, and neither call goes near the ACL —
	/// the ACL gates the `plugin:shell|*` IPC commands, which only the webview
	/// can issue. So `shell:allow-spawn` and `shell:allow-kill` in the
	/// capability bought the frontend the ability to run arbitrary programs and
	/// signal arbitrary pids, and bought this app nothing at all.
	///
	/// Nothing under `src/` imports `@tauri-apps/plugin-shell`, so removing them
	/// cost the frontend nothing either. The grant list is the app's whole
	/// attack surface from the webview inwards, and in a debug build the MCP
	/// bridge already hands an agent the run of that webview — so an unused
	/// grant is not neutral, and this is what keeps it from drifting back.
	#[test]
	fn the_webview_is_granted_no_shell_permission() {
		let capability: serde_json::Value =
			serde_json::from_str(include_str!("../capabilities/default.json"))
				.expect("capabilities/default.json must be valid JSON");
		let granted = capability["permissions"]
			.as_array()
			.expect("the capability must list permissions");
		let shell: Vec<&serde_json::Value> = granted
			.iter()
			.filter(|p| p.as_str().is_some_and(|p| p.starts_with("shell:")))
			.collect();
		assert!(
			shell.is_empty(),
			"the sidecar is spawned and killed from Rust, which bypasses the ACL — a \
			 shell grant here is reachable only by the webview; got {shell:?}"
		);
	}

	/// The script only assigns `window.__QUIVER__` on hostnames this app owns.
	/// `quiver.localhost` is one of them on Windows: it is the origin wry's
	/// work-around serves the scheme from, so a page loaded there would
	/// otherwise get no config at all.
	#[test]
	fn the_bootstrap_guard_covers_every_hostname_this_app_serves() {
		let script = quiver_bootstrap_script();
		for host in ["localhost", "tauri.localhost", "quiver.localhost"] {
			assert!(
				script.contains(&format!("'{host}'")),
				"the hostname guard must admit {host}; got {script}"
			);
		}
	}
}
