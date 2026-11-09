//! `quiver-arrow-window`: one arrow app in its own native WebView window.
//!
//! The host (the GPUI app) starts one of these per open arrow app and talks to it over stdin/stdout JSON
//! lines (see `quiver_arrow_host::message`). The window serves the `arrow-app` custom protocol by forwarding
//! each request to the host, which answers it through the same CSP / host allow-list code the Tauri app used.
//! This process holds no daemon credentials and never talks to the daemon itself.

mod ipc;

use quiver_arrow_host::{HostToWindow, ProtocolRequest, ProtocolResponse, WindowToHost, SCHEME};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::Response;
use wry::{RequestAsyncResponder, WebViewBuilder};

enum UserEvent {
    Host(HostToWindow),
}

type Pending = Arc<Mutex<HashMap<u64, RequestAsyncResponder>>>;

fn main() {
    let start_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{SCHEME}://about.invalid/"));
    let title = std::env::args().nth(2).unwrap_or_else(|| "Quiver".into());

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let Ok(window) = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(1000.0, 720.0))
        .build(&event_loop)
    else {
        eprintln!("arrow-window: cannot create a window");
        std::process::exit(1);
    };

    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
    let webview = {
        let (pending, next_id) = (pending.clone(), next_id.clone());
        let builder = WebViewBuilder::new()
            .with_asynchronous_custom_protocol(
                SCHEME.to_string(),
                move |_webview_id, request, responder| {
                    let id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let proto = ProtocolRequest {
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
                    if let Ok(mut map) = pending.lock() {
                        map.insert(id, responder);
                    }
                    ipc::send_to_host(&WindowToHost::ProtocolRequest(proto));
                },
            )
            .with_url(&start_url);
        build_webview(builder, &window)
    };
    let Ok(webview) = webview else {
        eprintln!("arrow-window: cannot create the webview");
        std::process::exit(1);
    };

    ipc::read_host_messages({
        let proxy = proxy.clone();
        move |msg| {
            let _ = proxy.send_event(UserEvent::Host(msg));
        }
    });
    ipc::send_to_host(&WindowToHost::Ready);

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Host(msg)) => match msg {
                HostToWindow::Navigate { url } => {
                    let _ = webview.load_url(&url);
                }
                HostToWindow::Reload => {
                    let _ = webview.evaluate_script("location.reload()");
                }
                HostToWindow::Title { title } => window.set_title(&title),
                HostToWindow::ProtocolResponse(resp) => respond(&pending, resp),
                HostToWindow::Close => {
                    ipc::send_to_host(&WindowToHost::Closed);
                    *flow = ControlFlow::Exit;
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                ipc::send_to_host(&WindowToHost::Closed);
                *flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Answers a pending request on the main thread (WebKit requires scheme tasks to finish there).
fn respond(pending: &Pending, resp: ProtocolResponse) {
    let Some(responder) = pending.lock().ok().and_then(|mut m| m.remove(&resp.id)) else {
        return;
    };
    let mut builder = Response::builder().status(resp.status);
    for (name, value) in resp.headers.iter() {
        builder = builder.header(name, value);
    }
    match builder.body(resp.body) {
        Ok(response) => responder.respond(response),
        Err(_) => responder.respond(
            Response::builder()
                .status(502)
                .body(Vec::new())
                .unwrap_or_default(),
        ),
    }
}

#[cfg(target_os = "linux")]
fn build_webview(
    builder: WebViewBuilder,
    window: &tao::window::Window,
) -> wry::Result<wry::WebView> {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;
    builder.build_gtk(window.default_vbox().ok_or(wry::Error::MessageSender)?)
}

#[cfg(not(target_os = "linux"))]
fn build_webview(
    builder: WebViewBuilder,
    window: &tao::window::Window,
) -> wry::Result<wry::WebView> {
    builder.build(window)
}
