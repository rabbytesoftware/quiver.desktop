//! The slice of the daemon's JSON envelope API the announce needs (port of
//! `apiRequest` / `apiFetch` in `src/lib/transport/api.ts`).

use http::Request;
use serde::Deserialize;

use crate::transport::Transport;
use crate::{Error, Result};

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    success: bool,
    error: Option<String>,
    data: Option<serde_json::Value>,
}

/// One call. Succeeds with the envelope's `data` (`None` for an empty answer).
///
/// An error status, or an envelope that is not `success`, is [`Error::Api`]
/// carrying the status: callers branch on it. A 204, or a 2xx with no
/// parseable body, is a success with no data. A 202 that does not carry a
/// success envelope is a bare acknowledgement.
pub(super) async fn call(
    transport: &dyn Transport,
    method: &str,
    path: &str,
    json: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>> {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("quiver://localhost{path}"));
    let body = match json {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            serde_json::to_vec(&value).map_err(|e| Error::Protocol(e.to_string()))?
        }
        None => Vec::new(),
    };
    let req = builder
        .body(body)
        .map_err(|e| Error::Protocol(e.to_string()))?;

    let resp = transport.request(req).await?;
    let status = resp.status();
    let envelope = serde_json::from_slice::<Envelope>(resp.body()).ok();

    if status.is_success() && (status.as_u16() == 204 || envelope.is_none()) {
        return Ok(None);
    }
    let success = envelope.as_ref().is_some_and(|e| e.success);
    if status.as_u16() == 202 && !success {
        return Ok(None);
    }
    if !status.is_success() || !success {
        let message = envelope.and_then(|e| e.error).unwrap_or_else(|| {
            format!(
                "{} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("")
            )
        });
        return Err(Error::Api {
            status: status.as_u16(),
            message,
        });
    }
    Ok(envelope.and_then(|e| e.data))
}

/// `GET` and decode `data` as `T`.
pub(super) async fn fetch<T: serde::de::DeserializeOwned>(
    transport: &dyn Transport,
    path: &str,
) -> Result<T> {
    let data = call(transport, "GET", path, None).await?;
    serde_json::from_value(data.unwrap_or(serde_json::Value::Null))
        .map_err(|e| Error::Protocol(format!("unexpected response from {path}: {e}")))
}
