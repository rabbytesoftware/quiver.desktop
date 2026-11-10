use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{mpsc, Notify};

use super::{ApiError, ApiResult, CloseInfo, CoreApi, CoreStream, Method, StreamEvent};

type Reply = Result<ApiResult<Option<Value>>, ApiError>;
type Responder =
    Arc<dyn Fn(Option<Value>) -> Pin<Box<dyn Future<Output = Reply> + Send>> + Send + Sync>;

/// One recorded call.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub method: Method,
    pub path: String,
    pub body: Option<Value>,
}

/// The test side of a stream the code under test opened.
#[derive(Clone)]
pub struct FakeStreamHandle {
    tx: mpsc::UnboundedSender<StreamEvent>,
}

impl FakeStreamHandle {
    pub fn send_text(
        &self,
        text: impl Into<String>,
    ) {
        let _ = self.tx.send(StreamEvent::Message(text.into()));
    }

    pub fn send_json(
        &self,
        value: Value,
    ) {
        self.send_text(value.to_string());
    }

    pub fn close(
        &self,
        info: Option<(u16, &str)>,
    ) {
        let info = info.map(|(code, reason)| CloseInfo {
            code,
            reason: reason.into(),
        });
        let _ = self.tx.send(StreamEvent::Closed(info));
    }

    /// Whether the consumer dropped (closed) its end.
    pub fn is_dropped(&self) -> bool {
        self.tx.is_closed()
    }
}

struct FakeStream {
    rx: mpsc::UnboundedReceiver<StreamEvent>,
}

#[async_trait]
impl CoreStream for FakeStream {
    async fn next(&mut self) -> StreamEvent {
        self.rx.recv().await.unwrap_or(StreamEvent::Closed(None))
    }
}

#[derive(Default)]
struct Inner {
    routes: HashMap<(Method, String), Responder>,
    calls: Vec<Call>,
    streams: Vec<(String, FakeStreamHandle)>,
    failing_opens: HashMap<String, u32>,
    reachable: bool,
}

/// In-memory [`CoreApi`] for tests: canned routes, a call log, and streams the test drives by hand.
#[derive(Clone, Default)]
pub struct FakeCoreApi {
    inner: Arc<Mutex<Inner>>,
    opened: Arc<Notify>,
}

impl FakeCoreApi {
    pub fn new() -> Self {
        let fake = Self::default();
        fake.lock().reachable = true;
        fake
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Answers `method path` with `data` and a 200.
    pub fn route(
        &self,
        method: Method,
        path: &str,
        data: Value,
    ) {
        self.route_status(method, path, 200, Some(data));
    }

    pub fn route_status(
        &self,
        method: Method,
        path: &str,
        status: u16,
        data: Option<Value>,
    ) {
        self.route_fn(method, path, move |_| {
            let data = data.clone();
            async move { Ok(ApiResult { status, data }) }
        });
    }

    pub fn route_error(
        &self,
        method: Method,
        path: &str,
        status: u16,
        message: &str,
    ) {
        let message = message.to_string();
        self.route_fn(method, path, move |_| {
            let message = message.clone();
            async move { Err(ApiError::new(message, status)) }
        });
    }

    /// Answers through a future, so a test can park the reply until it is ready (see `Gate`).
    pub fn route_fn<F, Fut>(
        &self,
        method: Method,
        path: &str,
        f: F,
    ) where
        F: Fn(Option<Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Reply> + Send + 'static,
    {
        let responder: Responder = Arc::new(move |body| Box::pin(f(body)));
        self.lock()
            .routes
            .insert((method, path.to_string()), responder);
    }

    pub fn set_reachable(
        &self,
        reachable: bool,
    ) {
        self.lock().reachable = reachable;
    }

    /// The next `count` opens of `path` fail to connect.
    pub fn fail_opens(
        &self,
        path: &str,
        count: u32,
    ) {
        self.lock().failing_opens.insert(path.into(), count);
    }

    pub fn calls(&self) -> Vec<Call> {
        self.lock().calls.clone()
    }

    pub fn calls_to(
        &self,
        method: Method,
        path: &str,
    ) -> usize {
        self.lock()
            .calls
            .iter()
            .filter(|c| c.method == method && c.path == path)
            .count()
    }

    /// How many streams were opened on `path` so far.
    pub fn open_count(
        &self,
        path: &str,
    ) -> usize {
        self.lock()
            .streams
            .iter()
            .filter(|(p, _)| p == path)
            .count()
    }

    /// Waits (on a signal, never a timer) until the `index`th stream of `path` has been opened.
    pub async fn wait_stream(
        &self,
        path: &str,
        index: usize,
    ) -> FakeStreamHandle {
        loop {
            let notified = self.opened.notified();
            if let Some(handle) = self.stream(path, index) {
                return handle;
            }
            notified.await;
        }
    }

    pub fn stream(
        &self,
        path: &str,
        index: usize,
    ) -> Option<FakeStreamHandle> {
        self.lock()
            .streams
            .iter()
            .filter(|(p, _)| p == path)
            .nth(index)
            .map(|(_, h)| h.clone())
    }
}

#[async_trait]
impl CoreApi for FakeCoreApi {
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Reply {
        let responder = {
            let mut inner = self.lock();
            inner.calls.push(Call {
                method,
                path: path.to_string(),
                body: body.clone(),
            });
            inner.routes.get(&(method, path.to_string())).cloned()
        };
        match responder {
            Some(r) => r(body).await,
            None => Err(ApiError::new(format!("no route for {method} {path}"), 404)),
        }
    }

    async fn open_stream(
        &self,
        path: &str,
    ) -> Result<Box<dyn CoreStream>, ApiError> {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut inner = self.lock();
            if let Some(n) = inner.failing_opens.get_mut(path) {
                if *n > 0 {
                    *n -= 1;
                    inner
                        .streams
                        .push((path.to_string(), FakeStreamHandle { tx }));
                    drop(inner);
                    self.opened.notify_waiters();
                    return Err(ApiError::transport("connection refused"));
                }
            }
            inner
                .streams
                .push((path.to_string(), FakeStreamHandle { tx }));
        }
        self.opened.notify_waiters();
        Ok(Box::new(FakeStream { rx }))
    }

    async fn is_reachable(&self) -> bool {
        self.lock().reachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CoreApiExt;

    #[tokio::test]
    async fn routes_answer_and_calls_are_logged() {
        let api = FakeCoreApi::new();
        api.route(Method::Get, "/v0/x", serde_json::json!({"a": 1}));
        assert_eq!(
            api.get_json("/v0/x").await.unwrap(),
            serde_json::json!({"a": 1})
        );
        assert_eq!(api.get_json("/v0/missing").await.unwrap_err().status, 404);
        assert_eq!(api.calls().len(), 2);
    }

    #[tokio::test]
    async fn streams_are_driven_by_the_test() {
        let api = FakeCoreApi::new();
        let mut stream = api.open_stream("/v0/arrow").await.unwrap();
        let handle = api.wait_stream("/v0/arrow", 0).await;
        handle.send_text("hi");
        handle.close(Some((1000, "completed")));
        assert_eq!(stream.next().await, StreamEvent::Message("hi".into()));
        assert_eq!(
            stream.next().await,
            StreamEvent::Closed(Some(CloseInfo {
                code: 1000,
                reason: "completed".into()
            }))
        );
    }
}
