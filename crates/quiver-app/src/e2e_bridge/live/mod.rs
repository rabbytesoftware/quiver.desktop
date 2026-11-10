//! The real bridge: a tokio server (from `quiver-e2e`) plus a GPUI-thread command runner.

mod attach;
mod commands;
mod driver;
mod input;
mod logs;
mod os_mouse;
mod screenshot;
#[cfg(target_os = "linux")]
mod screenshot_pages;

use gpui::{AnyWindowHandle, App};
use quiver_e2e::server::{serve, BridgeHandles};
use serde_json::Value;
use std::sync::{Arc, LazyLock};

pub use attach::attach;
pub use logs::install_logger;

/// Produces the JSON the `state.dump` call returns.
pub type StateSnapshot = Arc<dyn Fn() -> Value + Send + Sync>;

/// Process-wide handles: created on first use so a signal raised before [`start`] is latched, not lost.
static HANDLES: LazyLock<BridgeHandles> = LazyLock::new(BridgeHandles::new);

/// Raises a named signal for `wait.signal` (latched).
pub fn signal(name: &str) {
    HANDLES.signals.raise(name);
}

/// Call once per painted root frame: refreshes the element registry and wakes `wait.node` watchers.
pub fn frame_painted() {
    let frame = quiver_ui::inspect::begin_frame();
    if frame == 1 {
        signal("first-frame");
    }
    HANDLES.frames.send_replace(frame);
}

/// Marks the current frame as completely painted and wakes `wait.node` watchers: the registry is exact from here
/// until the next frame starts (nodes register while painting, which is after the frame-start wake-up).
fn frame_settled() {
    quiver_ui::inspect::settle();
    HANDLES.frames.send_replace(quiver_ui::inspect::frame());
}

/// An invisible element that must be the LAST thing painted in the root view: its paint reports the frame settled.
/// Deferred with the highest priority so it also comes after any deferred layer (popups, tooltips).
pub fn settle_marker() -> impl gpui::IntoElement {
    use gpui::{canvas, deferred, px, Styled as _};
    deferred(
        canvas(|_, _, _| (), |_, _, _, _| frame_settled())
            .absolute()
            .size(px(0.)),
    )
    .with_priority(usize::MAX)
}

/// Starts the bridge if `QUIVER_E2E_PORT` is set. Must be called on the GPUI thread with the main window.
pub(crate) fn start(
    cx: &mut App,
    window: AnyWindowHandle,
    runtime: tokio::runtime::Handle,
    state: StateSnapshot,
) {
    let Some(port) = std::env::var(quiver_e2e::protocol::PORT_ENV)
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return;
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let driver = Arc::new(driver::GpuiDriver::new(tx, state));
    runtime.spawn(async move {
        if let Err(e) = serve(port, driver, HANDLES.clone()).await {
            log::error!("e2e bridge stopped: {e}");
        }
    });
    commands::run(cx, window, rx);
}
