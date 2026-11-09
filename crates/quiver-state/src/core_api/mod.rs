//! The seam between state and the daemon. quiver-state only talks to `dyn CoreApi`; the app wires
//! it to `quiver-core-client` (HTTP/WS over the right transport). Mirrors `src/lib/transport/api.ts`.

use std::fmt;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[cfg(any(test, feature = "testing"))]
mod fake;
#[cfg(any(test, feature = "testing"))]
pub use fake::{FakeCoreApi, FakeStreamHandle};

/// A failed daemon call: the HTTP status (0 when the request never got an answer) and the message
/// from the envelope's `error` (or the status line).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn new(message: impl Into<String>, status: u16) -> Self {
        Self {
            message: message.into(),
            status,
        }
    }

    /// A failure before any HTTP answer (connect refused, transport gone).
    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(message, 0)
    }

    pub fn is_not_found(&self) -> bool {
        self.status == 404
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        })
    }
}

/// The unwrapped envelope with the status kept beside the data, for the calls whose success
/// statuses mean different things (`POST /v0/runtime/:ns/update`: 202 started, 200 nothing newer).
#[derive(Debug, Clone, PartialEq)]
pub struct ApiResult<T> {
    pub status: u16,
    pub data: T,
}

/// What the daemon said when it closed a stream; absent when the transport could not tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseInfo {
    pub code: u16,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// One text frame.
    Message(String),
    /// The stream ended; no further event follows.
    Closed(Option<CloseInfo>),
}

/// A WebSocket-like stream. Dropping it closes it.
#[async_trait]
pub trait CoreStream: Send {
    async fn next(&mut self) -> StreamEvent;
}

#[async_trait]
pub trait CoreApi: Send + Sync {
    /// One daemon call, envelope already unwrapped (see `transport::api::api_request`). `data` is
    /// `None` for a 204 or a bare acknowledgement.
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<ApiResult<Option<Value>>, ApiError>;

    /// Opens the stream at `path` (e.g. `/v0/arrow`). An `Err` means the connection could not be made.
    async fn open_stream(&self, path: &str) -> Result<Box<dyn CoreStream>, ApiError>;

    /// `GET /v0/health` answered 2xx. Probed once, never retried.
    async fn is_reachable(&self) -> bool;
}

/// Typed conveniences over [`CoreApi`] (a trait object cannot carry generic methods).
#[async_trait]
pub trait CoreApiExt: CoreApi {
    async fn get_json(&self, path: &str) -> Result<Value, ApiError> {
        Ok(self
            .request(Method::Get, path, None)
            .await?
            .data
            .unwrap_or(Value::Null))
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Option<Value>, ApiError> {
        Ok(self.request(method, path, body).await?.data)
    }

    /// `GET path` decoded into `T`. A missing payload decodes from `null`.
    async fn get<T: DeserializeOwned + Send>(&self, path: &str) -> crate::Result<T> {
        let value = self.get_json(path).await?;
        serde_json::from_value(value).map_err(|e| crate::Error::decode("response", e))
    }
}

impl<T: CoreApi + ?Sized> CoreApiExt for T {}
