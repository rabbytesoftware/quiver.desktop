use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::{Backend, ConnectionsSnapshot};
use crate::domain::{local_connection, ConnectionConfig, ConnectionStatus};
use crate::{Error, Result};

type SnapshotFuture = Pin<Box<dyn Future<Output = Result<ConnectionsSnapshot>> + Send>>;
type SnapshotFn = Arc<dyn Fn() -> SnapshotFuture + Send + Sync>;

struct Inner {
    get_connections: SnapshotFn,
    status: Vec<UnboundedSender<ConnectionStatus>>,
    changed: Vec<UnboundedSender<ConnectionsSnapshot>>,
    calls: Vec<String>,
}

/// In-memory [`Backend`] for tests: a canned connection list, hand-emitted events, a call log.
pub struct FakeBackend {
    inner: Mutex<Inner>,
}

impl Default for FakeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBackend {
    pub fn new() -> Self {
        let snapshot = ConnectionsSnapshot { connections: vec![local_connection()], active_id: "local".into() };
        let fake = Self {
            inner: Mutex::new(Inner {
                get_connections: Arc::new(|| Box::pin(async { Err(Error::Backend("unset".into())) })),
                status: vec![],
                changed: vec![],
                calls: vec![],
            }),
        };
        fake.set_connections(snapshot);
        fake
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_connections(&self, snapshot: ConnectionsSnapshot) {
        self.set_get_connections(move || {
            let snapshot = snapshot.clone();
            async move { Ok(snapshot) }
        });
    }

    /// Answers `get_connections` through a future, so a test can park it (race tests).
    pub fn set_get_connections<F, Fut>(&self, f: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ConnectionsSnapshot>> + Send + 'static,
    {
        self.lock().get_connections = Arc::new(move || Box::pin(f()));
    }

    pub fn fail_get_connections(&self) {
        self.set_get_connections(|| async { Err(Error::Backend("ipc gone".into())) });
    }

    pub fn emit_status(&self, status: ConnectionStatus) {
        self.lock().status.retain(|tx| tx.send(status).is_ok());
    }

    pub fn emit_connections(&self, snapshot: ConnectionsSnapshot) {
        self.lock().changed.retain(|tx| tx.send(snapshot.clone()).is_ok());
    }

    pub fn calls(&self) -> Vec<String> {
        self.lock().calls.clone()
    }

    pub fn call_count(&self, name: &str) -> usize {
        self.lock().calls.iter().filter(|c| c.as_str() == name).count()
    }

    fn record(&self, call: String) {
        self.lock().calls.push(call);
    }
}

#[async_trait]
impl Backend for FakeBackend {
    async fn get_connections(&self) -> Result<ConnectionsSnapshot> {
        let f = {
            let mut inner = self.lock();
            inner.calls.push("get_connections".into());
            inner.get_connections.clone()
        };
        f().await
    }

    fn on_core_status(&self) -> UnboundedReceiver<ConnectionStatus> {
        let (tx, rx) = unbounded_channel();
        self.lock().status.push(tx);
        rx
    }

    fn on_connections_changed(&self) -> UnboundedReceiver<ConnectionsSnapshot> {
        let (tx, rx) = unbounded_channel();
        self.lock().changed.push(tx);
        rx
    }

    async fn get_platform(&self) -> Result<String> {
        Ok("linux/amd64".into())
    }

    async fn check_remote_health(&self, url: &str) -> Result<()> {
        self.record(format!("check_remote_health {url}"));
        Ok(())
    }

    async fn add_connection(&self, name: &str, url: &str, code: &str) -> Result<ConnectionConfig> {
        self.record(format!("add_connection {name} {url} {code}"));
        Ok(ConnectionConfig {
            id: "new".into(),
            name: name.into(),
            kind: crate::domain::ConnectionKind::Remote,
            url: Some(url.into()),
            api_version: "v0".into(),
        })
    }

    async fn remove_connection(&self, id: &str) -> Result<()> {
        self.record(format!("remove_connection {id}"));
        Ok(())
    }

    async fn switch_connection(&self, id: &str) -> Result<()> {
        self.record(format!("switch_connection {id}"));
        Ok(())
    }

    async fn rename_connection(&self, id: &str, name: &str) -> Result<()> {
        self.record(format!("rename_connection {id} {name}"));
        Ok(())
    }
}
