//! The Tauri-facing half of the console's command runner: the two commands the
//! webview invokes. Everything they do lives in `crate::console`, where it is
//! tested against stand-in transports; what is left here needs a live
//! `AppHandle` and is excluded from coverage along with the other commands.

use tauri::ipc::Channel;
use tauri::State;

use crate::connection::ConnectionManager;
use crate::console::{check_line, ConsoleExecManager, EXEC_DEADLINE};

/// Run a console command on the active daemon and stream its frames to
/// `on_frame`. Returns as soon as the run has started; the frames, ending in
/// an `exit` or `error` frame, arrive on the channel.
#[tauri::command]
pub async fn console_exec(
	exec_id: String,
	line: String,
	on_frame: Channel<String>,
	connections: State<'_, ConnectionManager>,
	execs: State<'_, ConsoleExecManager>,
) -> Result<(), String> {
	check_line(&line)?;
	let transport = connections.transport().await;
	// The run belongs to the manager now; its handle is only for tests.
	drop(execs.start(exec_id, transport, line, on_frame, EXEC_DEADLINE));
	Ok(())
}

/// Cancel a run started by [`console_exec`].
#[tauri::command]
pub async fn console_exec_cancel(
	exec_id: String,
	execs: State<'_, ConsoleExecManager>,
) -> Result<(), String> {
	execs.cancel(&exec_id);
	Ok(())
}
