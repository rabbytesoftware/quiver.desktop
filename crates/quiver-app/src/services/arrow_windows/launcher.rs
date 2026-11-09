//! How a helper window process is started, as a seam: the real launcher spawns the
//! `quiver-arrow-window` binary and speaks JSON lines on its stdin/stdout; a test hands the
//! supervisor in-memory channels instead.

use quiver_arrow_host::{HostToWindow, WindowToHost};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::services::Result;

/// What the helper is started with: the page to load and the window title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperArgs {
    pub url: String,
    pub title: String,
}

/// A running helper. Dropping it ends the helper (and so its window): the real one is killed, so
/// no window can outlive the host.
pub struct HelperLink {
    pub to_helper: UnboundedSender<HostToWindow>,
    /// Ends when the helper exits, however it does.
    pub from_helper: UnboundedReceiver<WindowToHost>,
    pub guard: Box<dyn Send>,
}

pub trait HelperLauncher: Send + Sync {
    /// Must be called inside the async runtime (the real launcher spawns tasks).
    fn launch(&self, args: &HelperArgs) -> Result<HelperLink>;
}
