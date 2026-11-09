//! Running one command against the daemon's exec route and relaying its frames.

use std::time::Duration;

use ::http::Request;

use super::splitter::NdjsonSplitter;
use crate::bridge::FrameSink;
use crate::transport::Transport;

/// The exec route.
const EXEC_PATH: &str = "/v0/console/exec";

/// How much of an error response body is read to find its message.
pub(super) const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

/// `{"type":"error",...}`: a failure that is not the command's own exit.
pub(super) fn error_frame(status: u16, message: &str) -> String {
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

fn exec_request(line: &str) -> Request<Vec<u8>> {
    // Neither step can fail: a JSON object of one string always serialises, and the
    // builder is given a fixed method, a well-formed URI and valid header names.
    // The error arms exist so that no panic path is left in.
    let body = serde_json::to_vec(&serde_json::json!({ "line": line })).unwrap_or_default();
    Request::builder()
        .method("POST")
        .uri(format!("quiver://localhost{EXEC_PATH}"))
        .header("content-type", "application/json")
        .header("accept", "application/x-ndjson")
        .body(body)
        .unwrap_or_else(|_| Request::new(Vec::new()))
}

/// Run `line` on `transport`, pushing every frame to `sink`, until the
/// command exits or the connection ends.
///
/// The whole of the exec path, so it can be driven
/// against a real server or a stand-in transport.
pub async fn run_exec<S: FrameSink>(
    transport: &dyn Transport,
    line: &str,
    sink: &S,
    deadline: Duration,
) {
    let request = exec_request(line);
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
        let why = match failure {
            Some(why) => why,
            None => "the connection ended before the command finished".into(),
        };
        sink.send(error_frame(0, &why));
    }
}
