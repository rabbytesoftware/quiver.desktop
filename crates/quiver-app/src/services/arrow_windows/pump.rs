//! Serves one open page: answers its `arrow-app` requests, relays its WebSockets, and reports when it is gone.

use std::sync::{Arc, Weak};

use quiver_arrow_host::{HostToWindow, ProtocolService, WindowToHost, WsSession};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinSet;

use super::recorder::ProtocolLog;
use super::Inner;

pub(super) struct Pump {
    pub inner: Weak<Inner>,
    pub protocol: ProtocolService,
    pub sockets: WsSession,
    /// Messages for the page's shim, produced by `sockets`.
    pub to_page: UnboundedReceiver<String>,
    pub log: Arc<ProtocolLog>,
    pub namespace: String,
    pub generation: u64,
    pub from_helper: UnboundedReceiver<WindowToHost>,
    pub to_helper: UnboundedSender<HostToWindow>,
}

impl Pump {
    /// Runs until the page says it closed or its channel ends.
    pub async fn run(mut self) {
        // Dropped with the pump, which aborts any request still being answered.
        let mut answering = JoinSet::new();
        loop {
            let message = tokio::select! {
                message = self.from_helper.recv() => match message {
                    Some(message) => message,
                    None => break,
                },
                Some(json) = self.to_page.recv() => {
                    let _ = self.to_helper.send(HostToWindow::Deliver { json });
                    continue;
                }
            };
            match message {
                WindowToHost::Ready => log::debug!("arrow page for {} is ready", self.namespace),
                WindowToHost::Closed => break,
                WindowToHost::Shim { json } => self.sockets.handle(&json),
                WindowToHost::ProtocolRequest(request) => {
                    let protocol = self.protocol.clone();
                    let to_helper = self.to_helper.clone();
                    let (log, namespace) = (self.log.clone(), self.namespace.clone());
                    answering.spawn(async move {
                        let response = protocol.answer(request.clone()).await;
                        log.record(&namespace, &request, &response);
                        let _ = to_helper.send(HostToWindow::ProtocolResponse(response));
                    });
                }
            }
        }
        if let Some(inner) = self.inner.upgrade() {
            inner.window_gone(&self.namespace, self.generation);
        }
    }
}
