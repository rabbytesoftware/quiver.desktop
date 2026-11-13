//! Arrow apps inside the main window. Each open app is a webview that is a child of the main window and
//! sits over the pane body; the supervisor ([`super::ArrowWindows`]) keeps talking to it through the same
//! message channels it always used (`HostToWindow` / `WindowToHost`), so the app cap, Reload, Stop, the
//! protocol rules and the e2e state dump are unchanged.
//!
//! Webviews belong to one thread: GTK's own on Linux, the UI thread on macOS and Windows. This module posts
//! jobs to it and never touches a webview anywhere else.

mod pages;
mod parent;
#[cfg(target_os = "linux")]
mod thread_gtk;
#[cfg(not(target_os = "linux"))]
mod thread_ui;
#[cfg(target_os = "linux")]
mod x11_lookup;

use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "linux")]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

use quiver_arrow_host::{HostToWindow, WindowToHost};
use raw_window_handle::HasWindowHandle;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;

use super::launcher::{HelperArgs, HelperLauncher, HelperLink};
#[cfg(target_os = "linux")]
use crate::services::Error;
use crate::services::Result;
use pages::Pages;
pub use parent::ParentHandle;

/// The main window, once it exists. Pages are created inside it.
pub(crate) type ParentSlot = Arc<Mutex<Option<ParentHandle>>>;
/// Work for the page thread. It gets the main window's handle when there is one.
pub(crate) type Job = Box<dyn FnOnce(&mut Pages, Option<&ParentHandle>) + Send>;

pub(crate) trait PageThread: Send + Sync {
    fn post(
        &self,
        job: Job,
    );
}

/// Creates the arrow apps' pages inside the main window.
pub struct EmbeddedPages {
    parent: ParentSlot,
    next_id: AtomicU64,
    #[cfg(target_os = "linux")]
    thread: OnceLock<std::result::Result<Arc<thread_gtk::GtkThread>, String>>,
    #[cfg(not(target_os = "linux"))]
    thread: Arc<thread_ui::UiThread>,
}

impl EmbeddedPages {
    pub fn new() -> Self {
        Self {
            parent: Arc::default(),
            next_id: AtomicU64::new(1),
            #[cfg(target_os = "linux")]
            thread: OnceLock::new(),
            #[cfg(not(target_os = "linux"))]
            thread: Arc::new(thread_ui::UiThread::new()),
        }
    }

    /// Tells the pages which window they live in. Call on the UI thread when the main window opens.
    pub fn attach(
        &self,
        window: &impl HasWindowHandle,
    ) {
        let handle = ParentHandle::of(window);
        if handle.is_none() {
            log::error!("the main window has no native handle: arrow apps cannot be shown");
        }
        if let Ok(mut slot) = self.parent.lock() {
            *slot = handle;
        }
    }

    /// Starts whatever the platform needs on the UI thread (macOS, Windows: the job runner).
    pub fn start(
        &self,
        cx: &mut gpui::App,
    ) {
        #[cfg(not(target_os = "linux"))]
        self.thread.pump(self.parent.clone(), cx);
        #[cfg(target_os = "linux")]
        let _ = cx;
    }

    fn page_thread(&self) -> Result<Arc<dyn PageThread>> {
        #[cfg(target_os = "linux")]
        {
            let started = self
                .thread
                .get_or_init(|| thread_gtk::GtkThread::start(self.parent.clone()).map(Arc::new));
            match started {
                Ok(thread) => Ok(thread.clone() as Arc<dyn PageThread>),
                Err(e) => Err(Error::Helper(e.clone())),
            }
        }
        #[cfg(not(target_os = "linux"))]
        Ok(self.thread.clone() as Arc<dyn PageThread>)
    }
}

impl Default for EmbeddedPages {
    fn default() -> Self {
        Self::new()
    }
}

/// Closing the page when the supervisor lets go of it.
struct PageGuard {
    thread: Arc<dyn PageThread>,
    id: u64,
    forwarder: JoinHandle<()>,
}

impl Drop for PageGuard {
    fn drop(&mut self) {
        self.forwarder.abort();
        let id = self.id;
        self.thread
            .post(Box::new(move |pages, _| pages.destroy(id)));
    }
}

impl HelperLauncher for EmbeddedPages {
    fn launch(
        &self,
        args: &HelperArgs,
    ) -> Result<HelperLink> {
        let thread = self.page_thread()?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (to_helper, mut commands) = unbounded_channel::<HostToWindow>();
        let (events, from_helper) = unbounded_channel::<WindowToHost>();

        let url = args.url.clone();
        let created = events.clone();
        thread.post(Box::new(move |pages, parent| {
            let made = match parent {
                Some(parent) => pages.create(id, parent, &url, created.clone()),
                None => Err("the main window is not open".to_owned()),
            };
            match made {
                Ok(()) => {
                    let _ = created.send(WindowToHost::Ready);
                }
                Err(e) => {
                    log::error!("could not open the arrow app's page: {e}");
                    let _ = created.send(WindowToHost::Closed);
                }
            }
        }));

        let relay = thread.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(command) = commands.recv().await {
                let last = command == HostToWindow::Close;
                let closed = events.clone();
                relay.post(Box::new(move |pages, _| {
                    pages.apply(id, command);
                    if last {
                        let _ = closed.send(WindowToHost::Closed);
                    }
                }));
                if last {
                    break;
                }
            }
        });
        Ok(HelperLink {
            to_helper,
            from_helper,
            guard: Box::new(PageGuard {
                thread,
                id,
                forwarder,
            }),
        })
    }
}
