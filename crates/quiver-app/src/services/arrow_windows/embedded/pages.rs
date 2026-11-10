//! The webviews themselves. A [`Pages`] lives on exactly one thread (the one that may touch webviews) and
//! is only reached through jobs posted to it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use quiver_arrow_host::{
    HostToWindow, PageBounds, ProtocolRequest, ProtocolResponse, WindowToHost, SCHEME,
};
use tokio::sync::mpsc::UnboundedSender;
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::http::Response;
use wry::{Rect, RequestAsyncResponder, WebView, WebViewBuilder};

use super::parent::ParentHandle;

type Pending = Arc<Mutex<HashMap<u64, RequestAsyncResponder>>>;

struct Page {
    view: WebView,
    pending: Pending,
}

#[derive(Default)]
pub(crate) struct Pages {
    views: HashMap<u64, Page>,
}

fn rect(b: PageBounds) -> Rect {
    Rect {
        position: LogicalPosition::new(f64::from(b.x), f64::from(b.y)).into(),
        size: LogicalSize::new(f64::from(b.width.max(1.0)), f64::from(b.height.max(1.0))).into(),
    }
}

impl Pages {
    /// Creates page `id` inside `parent`, hidden, loading `url`. Its protocol requests go to `events`.
    pub fn create(
        &mut self,
        id: u64,
        parent: &ParentHandle,
        url: &str,
        events: UnboundedSender<WindowToHost>,
    ) -> Result<(), String> {
        let pending: Pending = Arc::default();
        let next_request = Arc::new(AtomicU64::new(1));
        let waiting = pending.clone();
        let shim_events = events.clone();
        let view = WebViewBuilder::new()
            .with_visible(true)
            .with_bounds(rect(PageBounds {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }))
            .with_asynchronous_custom_protocol(SCHEME.to_string(), move |_, request, responder| {
                let id = next_request.fetch_add(1, Ordering::Relaxed);
                let forwarded = ProtocolRequest {
                    id,
                    method: request.method().to_string(),
                    uri: request.uri().to_string(),
                    headers: request
                        .headers()
                        .iter()
                        .filter_map(|(k, v)| {
                            v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
                        })
                        .collect(),
                    body: request.body().to_vec(),
                };
                if let Ok(mut map) = waiting.lock() {
                    map.insert(id, responder);
                }
                let _ = events.send(WindowToHost::ProtocolRequest(forwarded));
            })
            .with_ipc_handler(move |request| {
                let _ = shim_events.send(WindowToHost::Shim {
                    json: request.body().clone(),
                });
            })
            .with_url(url)
            .build_as_child(parent)
            .map_err(|e| e.to_string())?;
        self.views.insert(id, Page { view, pending });
        Ok(())
    }

    /// Applies one host message. False once the page is gone (it was asked to close).
    pub fn apply(
        &mut self,
        id: u64,
        message: HostToWindow,
    ) -> bool {
        let Some(page) = self.views.get(&id) else {
            return false;
        };
        match message {
            HostToWindow::Navigate { url } => {
                let _ = page.view.load_url(&url);
            }
            HostToWindow::Reload => {
                let _ = page.view.evaluate_script("location.reload()");
            }
            HostToWindow::Title { .. } => {}
            HostToWindow::Bounds(b) => {
                let _ = page.view.set_bounds(rect(b));
            }
            HostToWindow::Visible { visible } => {
                let _ = page.view.set_visible(visible);
                if !visible {
                    let _ = page.view.focus_parent();
                }
            }
            HostToWindow::ProtocolResponse(response) => respond(&page.pending, response),
            HostToWindow::Deliver { json } => {
                if let Ok(literal) = serde_json::to_string(&json) {
                    let _ = page
                        .view
                        .evaluate_script(&format!("window.__arrowShimReceive({literal})"));
                }
            }
            HostToWindow::Close => {
                self.destroy(id);
                return false;
            }
        }
        true
    }

    pub fn destroy(
        &mut self,
        id: u64,
    ) {
        if let Some(page) = self.views.remove(&id) {
            let _ = page.view.focus_parent();
        }
    }
}

/// Answers a pending request (on the page thread, where WebKit wants scheme tasks finished).
fn respond(
    pending: &Pending,
    response: ProtocolResponse,
) {
    let Some(responder) = pending
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&response.id))
    else {
        return;
    };
    let mut builder = Response::builder().status(response.status);
    for (name, value) in response.headers.iter() {
        builder = builder.header(name, value);
    }
    match builder.body(response.body) {
        Ok(ok) => responder.respond(ok),
        Err(_) => responder.respond(
            Response::builder()
                .status(502)
                .body(Vec::new())
                .unwrap_or_default(),
        ),
    }
}
