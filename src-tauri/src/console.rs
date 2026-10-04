//! Running a console command on the active daemon, and streaming its output.
//!
//! `POST /v0/console/exec` answers with NDJSON: `out` frames as the command
//! writes, then exactly one `exit` frame (see `docs/console-spec.md`). The
//! webview cannot read a streamed body through the `quiver://` proxy, which
//! returns a response whole, so Rust is the HTTP client here: it dials the
//! active connection's own transport, splits the body into lines and pushes
//! each line, verbatim, down a Tauri `Channel` the frontend supplies.
//!
//! What this module does NOT do is interpret a command. The line is handed to
//! the daemon exactly as typed; the daemon owns the grammar, the allow-list and
//! every limit. This side only moves bytes, bounds its own memory, and makes
//! sure a frame is never lost silently:
//!
//!   * a daemon that answers with an error status (400 empty line, 403 not
//!     allowed, 429 busy, 401) becomes one `error` frame carrying the status and
//!     the daemon's message;
//!   * a connection that ends before an `exit` frame arrives becomes one `error`
//!     frame saying so, so the console never waits on a command that is gone;
//!   * dropping a run (cancel, page reload) closes the connection, which is what
//!     cancels the command on the daemon's side.
//!
//! Frame contents are never logged here: they are command output and may carry
//! anything the user ran.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tauri::http::Request;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tokio::task::AbortHandle;

use crate::connection::bridge::FrameSink;
use crate::connection::transport::Transport;
use crate::connection::ConnectionManager;

/// The exec route.
const EXEC_PATH: &str = "/v0/console/exec";

/// Longest command line this side forwards. The daemon enforces 1024 bytes
/// itself; this is only a guard so a pathological paste cannot become a
/// megabyte IPC argument before the daemon gets to say no.
const MAX_LINE_BYTES: usize = 4096;

/// The daemon bounds a command at 10 minutes; this is the same bound plus a
/// little grace so the daemon's own timeout answer arrives before ours.
const EXEC_DEADLINE: Duration = Duration::from_secs(630);

/// Longest single NDJSON line accepted. A frame is a line of output wrapped in
/// a little JSON, so this is generous; anything longer is dropped with a marker
/// rather than buffered without bound.
const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// How much of an error response body is read to find its message.
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

/// Splits a byte stream into NDJSON lines.
///
/// Chunk boundaries are arbitrary, so a line (or a multi-byte character) may
/// straddle two chunks; the splitter holds the partial tail until its newline
/// arrives. A line over [`MAX_FRAME_BYTES`] is discarded up to its newline and
/// replaced by a marker frame, so memory stays bounded whatever the peer sends.
#[derive(Default)]
pub struct NdjsonSplitter {
	tail: Vec<u8>,
	discarding: bool,
}

/// The frame substituted for a line that was too long to keep.
fn oversize_marker() -> String {
	serde_json::json!({
		"type": "out",
		"stream": "stderr",
		"data": "[quiver: a line of output was too long to display and was dropped]\n",
	})
	.to_string()
}

impl NdjsonSplitter {
	pub fn new() -> Self {
		Self::default()
	}

	/// Feed one chunk; returns the lines it completed, without their newline.
	/// Blank lines are skipped, and invalid UTF-8 is replaced rather than
	/// rejected: the frontend treats a non-JSON line as raw text.
	pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
		let mut lines = Vec::new();
		let mut rest = chunk;
		while !rest.is_empty() {
			match rest.iter().position(|b| *b == b'\n') {
				Some(i) => {
					let (head, tail) = rest.split_at(i);
					rest = &tail[1..];
					if self.discarding {
						self.discarding = false;
						self.tail.clear();
						lines.push(oversize_marker());
						continue;
					}
					self.tail.extend_from_slice(head);
					if self.tail.len() > MAX_FRAME_BYTES {
						self.tail.clear();
						lines.push(oversize_marker());
						continue;
					}
					if let Some(line) = Self::take(&mut self.tail) {
						lines.push(line);
					}
				}
				None => {
					if !self.discarding {
						self.tail.extend_from_slice(rest);
						if self.tail.len() > MAX_FRAME_BYTES {
							self.tail.clear();
							self.discarding = true;
						}
					}
					break;
				}
			}
		}
		lines
	}

	/// The unterminated remainder once the stream has ended, if any. A daemon
	/// that exits mid-line leaves one, and dropping it would hide the last
	/// words of the command.
	pub fn finish(&mut self) -> Option<String> {
		if self.discarding {
			self.discarding = false;
			self.tail.clear();
			return Some(oversize_marker());
		}
		Self::take(&mut self.tail)
	}

	fn take(buf: &mut Vec<u8>) -> Option<String> {
		let line = String::from_utf8_lossy(buf)
			.trim_end_matches('\r')
			.to_owned();
		buf.clear();
		(!line.trim().is_empty()).then_some(line)
	}
}

/// `{"type":"error",...}`: a failure that is not the command's own exit.
fn error_frame(status: u16, message: &str) -> String {
	serde_json::json!({"type": "error", "status": status, "message": message}).to_string()
}

/// The daemon's message out of a non-streaming error body
/// (`{"success":false,"error":"..."}`), falling back to the bare status.
fn error_message(status: u16, body: &[u8]) -> String {
	serde_json::from_slice::<serde_json::Value>(body)
		.ok()
		.and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_owned))
		.filter(|m| !m.is_empty())
		.unwrap_or_else(|| format!("the daemon answered {status}"))
}

fn is_exit_frame(line: &str) -> bool {
	serde_json::from_str::<serde_json::Value>(line)
		.ok()
		.and_then(|v| v.get("type").and_then(|t| t.as_str()).map(|t| t == "exit"))
		.unwrap_or(false)
}

fn exec_request(line: &str) -> Result<Request<Vec<u8>>, String> {
	let body = serde_json::to_vec(&serde_json::json!({ "line": line }))
		.map_err(|e| format!("encode command: {e}"))?;
	Request::builder()
		.method("POST")
		.uri(format!("quiver://localhost{EXEC_PATH}"))
		.header("content-type", "application/json")
		.header("accept", "application/x-ndjson")
		.body(body)
		.map_err(|e| format!("build request: {e}"))
}

/// Run `line` on `transport`, pushing every frame to `sink`, until the
/// command exits or the connection ends.
///
/// The whole of the exec path minus Tauri's `State`, so it can be driven
/// against a real server or a stand-in transport.
pub async fn run_exec<S: FrameSink>(
	transport: &dyn Transport,
	line: &str,
	sink: &S,
	deadline: Duration,
) {
	let request = match exec_request(line) {
		Ok(r) => r,
		Err(e) => return sink.send(error_frame(0, &e)),
	};
	let mut response = match transport.request_stream(request, deadline).await {
		Ok(r) => r,
		Err(e) => return sink.send(error_frame(0, &e.to_string())),
	};

	if response.status != 200 {
		let mut body = Vec::new();
		while let Some(Ok(chunk)) = response.chunks.recv().await {
			body.extend_from_slice(&chunk);
			if body.len() > MAX_ERROR_BODY_BYTES {
				break;
			}
		}
		return sink.send(error_frame(
			response.status,
			&error_message(response.status, &body),
		));
	}

	let mut splitter = NdjsonSplitter::new();
	let mut exited = false;
	let mut failure: Option<String> = None;
	while let Some(chunk) = response.chunks.recv().await {
		match chunk {
			Ok(bytes) => {
				for frame in splitter.push(&bytes) {
					exited |= is_exit_frame(&frame);
					sink.send(frame);
				}
			}
			Err(e) => {
				failure = Some(e.to_string());
				break;
			}
		}
	}
	if let Some(last) = splitter.finish() {
		exited |= is_exit_frame(&last);
		sink.send(last);
	}
	if !exited {
		sink.send(error_frame(
			0,
			&failure.unwrap_or_else(|| {
				"the connection ended before the command finished".into()
			}),
		));
	}
}

/// Tracks the runs in flight so each can be cancelled by id, and so a page
/// reload can end every one its outgoing page started.
///
/// Aborting a run's task drops its response, which closes the connection,
/// which cancels the command on the daemon.
#[derive(Default)]
pub struct ConsoleExecManager {
	runs: Mutex<HashMap<String, AbortHandle>>,
}

impl ConsoleExecManager {
	pub fn new() -> Self {
		Self::default()
	}

	fn register(&self, exec_id: String, handle: AbortHandle) {
		// A re-used id strands the previous run, so it is aborted.
		if let Some(previous) = self.runs.lock().unwrap().insert(exec_id, handle) {
			previous.abort();
		}
	}

	fn finish(&self, exec_id: &str) {
		self.runs.lock().unwrap().remove(exec_id);
	}

	/// Abort one run. Unknown ids are fine: the run may already have ended.
	pub fn cancel(&self, exec_id: &str) {
		if let Some(handle) = self.runs.lock().unwrap().remove(exec_id) {
			handle.abort();
		}
	}

	/// Abort every run: the page that started them is gone.
	pub fn cancel_all(&self) {
		for (_, handle) in self.runs.lock().unwrap().drain() {
			handle.abort();
		}
	}

	#[cfg(test)]
	fn in_flight(&self) -> usize {
		self.runs.lock().unwrap().len()
	}
}

/// Run a console command on the active daemon and stream its frames to
/// `on_frame`. Returns as soon as the run has started; the frames, ending in
/// an `exit` or `error` frame, arrive on the channel.
#[tauri::command]
pub async fn console_exec(
	exec_id: String,
	line: String,
	on_frame: Channel<String>,
	app: AppHandle,
	connections: State<'_, ConnectionManager>,
	execs: State<'_, ConsoleExecManager>,
) -> Result<(), String> {
	if line.len() > MAX_LINE_BYTES {
		return Err(format!("command is longer than {MAX_LINE_BYTES} bytes"));
	}
	// Resolved per call, so a command typed after a connection switch reaches
	// the new daemon with nothing to re-register.
	let transport = connections.transport().await;

	let id = exec_id.clone();
	let task = tokio::spawn(async move {
		run_exec(transport.as_ref(), &line, &on_frame, EXEC_DEADLINE).await;
		app.state::<ConsoleExecManager>().finish(&id);
	});
	execs.register(exec_id.clone(), task.abort_handle());
	// A command that finished before it was registered would otherwise stay in
	// the map for good.
	if task.is_finished() {
		execs.finish(&exec_id);
	}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::{StreamResponse, TransportError, WsStream};
	use async_trait::async_trait;
	use std::sync::{Arc, Mutex as StdMutex};
	use tauri::http::Response;
	use tokio::sync::mpsc;

	struct Collect(Arc<StdMutex<Vec<String>>>);
	impl FrameSink for Collect {
		fn send(&self, frame: String) {
			self.0.lock().unwrap().push(frame);
		}
	}
	fn sink() -> (Collect, Arc<StdMutex<Vec<String>>>) {
		let seen = Arc::new(StdMutex::new(Vec::new()));
		(Collect(Arc::clone(&seen)), seen)
	}

	/// A transport whose `request_stream` answers with a scripted status and
	/// chunks, and records the request it was given.
	struct Scripted {
		status: u16,
		chunks: Vec<Result<Vec<u8>, TransportError>>,
		seen: StdMutex<Option<(String, String, Vec<u8>)>>,
	}
	impl Scripted {
		fn new(status: u16, chunks: Vec<Result<Vec<u8>, TransportError>>) -> Self {
			Self {
				status,
				chunks,
				seen: StdMutex::new(None),
			}
		}
	}
	#[async_trait]
	impl Transport for Scripted {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			unreachable!("exec streams")
		}
		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
			unreachable!()
		}
		async fn request_stream(
			&self,
			req: Request<Vec<u8>>,
			_deadline: Duration,
		) -> Result<StreamResponse, TransportError> {
			*self.seen.lock().unwrap() = Some((
				req.method().to_string(),
				req.uri().path().to_string(),
				req.body().clone(),
			));
			let (tx, rx) = mpsc::channel(32);
			for c in &self.chunks {
				let item = match c {
					Ok(b) => Ok(b.clone()),
					Err(e) => Err(TransportError::Protocol(e.to_string())),
				};
				tx.try_send(item).unwrap();
			}
			Ok(StreamResponse {
				status: self.status,
				chunks: rx,
			})
		}
	}

	fn ok(s: &str) -> Result<Vec<u8>, TransportError> {
		Ok(s.as_bytes().to_vec())
	}

	// --- splitter -----------------------------------------------------------

	#[test]
	fn lines_split_on_newlines() {
		let mut s = NdjsonSplitter::new();
		assert_eq!(s.push(b"a\nb\n"), vec!["a", "b"]);
	}

	#[test]
	fn a_line_split_across_chunks_is_joined() {
		let mut s = NdjsonSplitter::new();
		assert!(s.push(b"{\"type\":\"o").is_empty());
		assert_eq!(s.push(b"ut\"}\n"), vec!["{\"type\":\"out\"}"]);
	}

	#[test]
	fn a_multibyte_character_split_across_chunks_survives() {
		let bytes = "héllo\n".as_bytes();
		let mut s = NdjsonSplitter::new();
		assert!(s.push(&bytes[..2]).is_empty());
		assert_eq!(s.push(&bytes[2..]), vec!["héllo"]);
	}

	#[test]
	fn crlf_and_blank_lines_are_tidied() {
		let mut s = NdjsonSplitter::new();
		assert_eq!(s.push(b"a\r\n\r\n\nb\n"), vec!["a", "b"]);
	}

	#[test]
	fn an_unterminated_tail_is_returned_at_the_end() {
		let mut s = NdjsonSplitter::new();
		assert!(s.push(b"last words").is_empty());
		assert_eq!(s.finish().as_deref(), Some("last words"));
		assert_eq!(s.finish(), None);
	}

	#[test]
	fn invalid_utf8_is_replaced_not_fatal() {
		let mut s = NdjsonSplitter::new();
		let lines = s.push(b"a\xffb\n");
		assert_eq!(lines, vec!["a\u{fffd}b"]);
	}

	#[test]
	fn an_oversize_line_is_replaced_by_a_marker_and_the_stream_recovers() {
		let mut s = NdjsonSplitter::new();
		let big = vec![b'x'; MAX_FRAME_BYTES + 10];
		// No newline yet: the splitter must already have stopped buffering.
		assert!(s.push(&big).is_empty());
		assert!(s.tail.is_empty(), "memory must stay bounded");
		let lines = s.push(b"more\nnext\n");
		assert_eq!(lines.len(), 2);
		assert!(lines[0].contains("too long"));
		assert_eq!(lines[1], "next");
	}

	#[test]
	fn an_oversize_line_that_ends_in_the_same_chunk_is_marked_too() {
		let mut s = NdjsonSplitter::new();
		let mut big = vec![b'x'; MAX_FRAME_BYTES + 1];
		big.push(b'\n');
		big.extend_from_slice(b"ok\n");
		let lines = s.push(&big);
		assert_eq!(lines.len(), 2);
		assert!(lines[0].contains("too long"));
		assert_eq!(lines[1], "ok");
	}

	#[test]
	fn an_oversize_line_cut_off_by_the_end_of_the_stream_is_marked() {
		let mut s = NdjsonSplitter::new();
		assert!(s.push(&vec![b'x'; MAX_FRAME_BYTES + 1]).is_empty());
		assert!(s.finish().unwrap().contains("too long"));
	}

	// --- run_exec -----------------------------------------------------------

	#[tokio::test]
	async fn frames_are_forwarded_verbatim_and_the_request_is_the_contract() {
		let t = Scripted::new(
			200,
			vec![
				ok("{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"hi\\n\"}\n{\"type\":"),
				ok("\"exit\",\"code\":0,\"error\":\"\"}\n"),
			],
		);
		let (sink, seen) = sink();
		run_exec(
			&t,
			"install github.com/char2cs/crowbar",
			&sink,
			Duration::from_secs(1),
		)
		.await;

		assert_eq!(
			*seen.lock().unwrap(),
			vec![
				"{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"hi\\n\"}",
				"{\"type\":\"exit\",\"code\":0,\"error\":\"\"}",
			]
		);
		let (method, path, body) = t.seen.lock().unwrap().clone().unwrap();
		assert_eq!(
			(method.as_str(), path.as_str()),
			("POST", "/v0/console/exec")
		);
		let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(
			body,
			serde_json::json!({"line": "install github.com/char2cs/crowbar"})
		);
	}

	#[tokio::test]
	async fn a_command_line_with_quotes_and_unicode_is_sent_as_one_json_string() {
		let t = Scripted::new(
			200,
			vec![ok("{\"type\":\"exit\",\"code\":0,\"error\":\"\"}\n")],
		);
		let (sink, _) = sink();
		let line = "run \"a b\" 'c' \u{1F600}\n\\";
		run_exec(&t, line, &sink, Duration::from_secs(1)).await;
		let (_, _, body) = t.seen.lock().unwrap().clone().unwrap();
		let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(body["line"], line);
	}

	#[tokio::test]
	async fn an_error_status_becomes_one_error_frame_with_the_daemons_message() {
		let t = Scripted::new(
			403,
			vec![ok("{\"success\":false,\"error\":\"command \\\"daemon\\\" is not available in the console\"}")],
		);
		let (sink, seen) = sink();
		run_exec(&t, "daemon", &sink, Duration::from_secs(1)).await;
		let frames = seen.lock().unwrap().clone();
		assert_eq!(frames.len(), 1);
		let f: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
		assert_eq!(f["type"], "error");
		assert_eq!(f["status"], 403);
		assert!(f["message"].as_str().unwrap().contains("not available"));
	}

	#[tokio::test]
	async fn an_error_status_with_an_unreadable_body_still_reports_the_status() {
		let t = Scripted::new(502, vec![ok("<html>bad gateway</html>")]);
		let (sink, seen) = sink();
		run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
		let f: serde_json::Value = serde_json::from_str(&seen.lock().unwrap()[0]).unwrap();
		assert_eq!(f["status"], 502);
		assert!(f["message"].as_str().unwrap().contains("502"));
	}

	#[tokio::test]
	async fn a_stream_that_ends_without_an_exit_frame_is_reported() {
		let t = Scripted::new(
			200,
			vec![ok(
				"{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"x\"}\n",
			)],
		);
		let (sink, seen) = sink();
		run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
		let frames = seen.lock().unwrap().clone();
		assert_eq!(frames.len(), 2);
		let f: serde_json::Value = serde_json::from_str(&frames[1]).unwrap();
		assert_eq!(f["type"], "error");
		assert!(f["message"].as_str().unwrap().contains("ended"));
	}

	#[tokio::test]
	async fn a_transport_error_mid_stream_is_reported_after_what_arrived() {
		let t = Scripted::new(
			200,
			vec![
				ok("{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"x\"}\n"),
				Err(TransportError::Protocol("connection reset".into())),
			],
		);
		let (sink, seen) = sink();
		run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
		let frames = seen.lock().unwrap().clone();
		assert_eq!(frames.len(), 2);
		let f: serde_json::Value = serde_json::from_str(&frames[1]).unwrap();
		assert_eq!(f["type"], "error");
		assert!(f["message"].as_str().unwrap().contains("connection reset"));
	}

	#[tokio::test]
	async fn an_exit_frame_without_a_trailing_newline_still_counts() {
		let t = Scripted::new(
			200,
			vec![ok("{\"type\":\"exit\",\"code\":1,\"error\":\"boom\"}")],
		);
		let (sink, seen) = sink();
		run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
		assert_eq!(
			seen.lock().unwrap().len(),
			1,
			"no extra error frame after a real exit"
		);
	}

	#[tokio::test]
	async fn a_transport_that_cannot_connect_reports_one_error_frame() {
		struct Dead;
		#[async_trait]
		impl Transport for Dead {
			async fn request(
				&self,
				_r: Request<Vec<u8>>,
			) -> Result<Response<Vec<u8>>, TransportError> {
				Err(TransportError::Connect("no socket".into()))
			}
			async fn open_ws(&self, _p: &str) -> Result<WsStream, TransportError> {
				Err(TransportError::Connect("no socket".into()))
			}
		}
		let (sink, seen) = sink();
		run_exec(&Dead, "list", &sink, Duration::from_secs(1)).await;
		let frames = seen.lock().unwrap().clone();
		assert_eq!(frames.len(), 1);
		assert!(frames[0].contains("no socket"));
	}

	// --- manager ------------------------------------------------------------

	#[tokio::test]
	async fn cancel_aborts_the_run_and_forgets_it() {
		let m = ConsoleExecManager::new();
		let task = tokio::spawn(std::future::pending::<()>());
		m.register("a".into(), task.abort_handle());
		assert_eq!(m.in_flight(), 1);
		m.cancel("a");
		assert_eq!(m.in_flight(), 0);
		assert!(task.await.unwrap_err().is_cancelled());
		m.cancel("a"); // unknown id: not an error
	}

	#[tokio::test]
	async fn cancel_all_aborts_every_run() {
		let m = ConsoleExecManager::new();
		let a = tokio::spawn(std::future::pending::<()>());
		let b = tokio::spawn(std::future::pending::<()>());
		m.register("a".into(), a.abort_handle());
		m.register("b".into(), b.abort_handle());
		m.cancel_all();
		assert_eq!(m.in_flight(), 0);
		assert!(a.await.unwrap_err().is_cancelled());
		assert!(b.await.unwrap_err().is_cancelled());
	}

	#[tokio::test]
	async fn a_reused_id_aborts_the_run_it_replaces() {
		let m = ConsoleExecManager::new();
		let first = tokio::spawn(std::future::pending::<()>());
		let second = tokio::spawn(std::future::pending::<()>());
		m.register("a".into(), first.abort_handle());
		m.register("a".into(), second.abort_handle());
		assert!(first.await.unwrap_err().is_cancelled());
		assert_eq!(m.in_flight(), 1);
		m.cancel_all();
	}

	#[tokio::test]
	async fn finish_forgets_without_aborting() {
		let m = ConsoleExecManager::new();
		let t = tokio::spawn(async {});
		m.register("a".into(), t.abort_handle());
		m.finish("a");
		assert_eq!(m.in_flight(), 0);
		t.await.unwrap();
	}
}
