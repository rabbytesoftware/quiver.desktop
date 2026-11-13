//! `quiver-state`'s [`CoreApi`] over whichever connection is ACTIVE.
//!
//! The transport is resolved on every call (like the `quiver://` proxy did), never cached, so a
//! connection switch takes effect on the very next request and stream.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use http::{header, Request};
use quiver_core_client::transport::WsStream;
use quiver_core_client::ConnectionManager;
use quiver_state::core_api::{
    ApiError, ApiResult, CloseInfo, CoreApi, CoreStream, Method, StreamEvent,
};
use quiver_state::transport::api::{
    api_request, core_is_reachable, RawFetch, RawResponse, RetryConfig, PROXY_ERROR_HEADER,
};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

/// Upper bound on one request, the `quiver://` proxy's own ceiling: a daemon that accepts and then
/// never answers must not pin a socket for the life of the app.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct TransportCoreApi {
    manager: Arc<ConnectionManager>,
    retry: RetryConfig,
    timeout: Duration,
}

impl TransportCoreApi {
    pub fn new(manager: Arc<ConnectionManager>) -> Self {
        Self {
            manager,
            retry: RetryConfig::default(),
            timeout: REQUEST_TIMEOUT,
        }
    }

    pub fn with_retry(
        mut self,
        retry: RetryConfig,
    ) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_timeout(
        mut self,
        timeout: Duration,
    ) -> Self {
        self.timeout = timeout;
        self
    }
}

fn http_method(method: Method) -> http::Method {
    match method {
        Method::Get => http::Method::GET,
        Method::Post => http::Method::POST,
        Method::Put => http::Method::PUT,
        Method::Patch => http::Method::PATCH,
        Method::Delete => http::Method::DELETE,
    }
}

/// What the Tauri proxy answered when its upstream failed: a 502 (504 on timeout) carrying the
/// `x-quiver-proxy` marker, which is exactly what makes `api_request` retry an idempotent read
/// through a cold start instead of failing it.
fn proxy_failure(
    status: u16,
    body: String,
) -> RawResponse {
    RawResponse {
        status,
        status_text: String::new(),
        proxy_error: true,
        body,
    }
}

#[async_trait]
impl RawFetch for TransportCoreApi {
    async fn fetch(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
    ) -> Result<RawResponse, ApiError> {
        let mut builder = Request::builder()
            .method(http_method(method))
            .uri(format!("quiver://localhost{path}"));
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        let request = builder
            .body(body.unwrap_or_default().into_bytes())
            .map_err(|e| ApiError::transport(format!("bad request {path}: {e}")))?;

        let manager = self.manager.clone();
        let call = async move { manager.transport().await.request(request).await };
        match tokio::time::timeout(self.timeout, call).await {
            Ok(Ok(resp)) => Ok(RawResponse {
                status: resp.status().as_u16(),
                status_text: resp
                    .status()
                    .canonical_reason()
                    .unwrap_or_default()
                    .to_string(),
                proxy_error: resp.headers().contains_key(PROXY_ERROR_HEADER),
                body: String::from_utf8_lossy(resp.body()).into_owned(),
            }),
            Ok(Err(e)) => Ok(proxy_failure(502, format!("quiver proxy: {e}"))),
            Err(_) => Ok(proxy_failure(
                504,
                "quiver.core did not answer in time".into(),
            )),
        }
    }
}

#[async_trait]
impl CoreApi for TransportCoreApi {
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<ApiResult<Option<Value>>, ApiError> {
        let body = body.map(|value| value.to_string());
        api_request(self, method, path, body, &self.retry).await
    }

    async fn open_stream(
        &self,
        path: &str,
    ) -> Result<Box<dyn CoreStream>, ApiError> {
        let transport = self.manager.transport().await;
        let socket = transport
            .open_ws(path)
            .await
            .map_err(|e| ApiError::transport(format!("open {path}: {e}")))?;
        Ok(Box::new(WsCoreStream {
            socket: Some(socket),
        }))
    }

    async fn is_reachable(&self) -> bool {
        core_is_reachable(self).await
    }
}

/// Text frames until the daemon ends the stream; dropping it closes the socket.
struct WsCoreStream {
    /// `None` once the stream has ended.
    socket: Option<WsStream>,
}

#[async_trait]
impl CoreStream for WsCoreStream {
    async fn next(&mut self) -> StreamEvent {
        let mut close_info = None;
        while let Some(socket) = self.socket.as_mut() {
            match socket.next().await {
                Some(Ok(Message::Text(text))) => return StreamEvent::Message(text),
                Some(Ok(Message::Close(frame))) => {
                    close_info = frame.map(|f| CloseInfo {
                        code: u16::from(f.code),
                        reason: f.reason.to_string(),
                    });
                }
                // Pings, pongs and binary frames are not DTOs; keep reading.
                Some(Ok(_)) => {}
                Some(Err(_)) | None => self.socket = None,
            }
        }
        StreamEvent::Closed(close_info)
    }
}
