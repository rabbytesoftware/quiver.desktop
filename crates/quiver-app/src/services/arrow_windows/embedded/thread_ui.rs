//! macOS and Windows: webviews are created on the UI thread (the thread GPUI runs on), as WKWebView and
//! WebView2 require. Jobs queue up and a foreground task of the app runs them.

use std::cell::RefCell;
use std::sync::Mutex;

use gpui::{App, AsyncApp};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::pages::Pages;
use super::{Job, PageThread, ParentSlot};

thread_local! {
    static PAGES: RefCell<Pages> = RefCell::new(Pages::default());
}

pub(super) struct UiThread {
    jobs: UnboundedSender<Job>,
    inbox: Mutex<Option<UnboundedReceiver<Job>>>,
}

impl UiThread {
    pub fn new() -> Self {
        let (jobs, inbox) = unbounded_channel();
        Self {
            jobs,
            inbox: Mutex::new(Some(inbox)),
        }
    }

    /// Starts the foreground task that runs the queued jobs. Call once, on the UI thread.
    pub fn pump(&self, parent: ParentSlot, cx: &mut App) {
        let Some(mut inbox) = self.inbox.lock().ok().and_then(|mut i| i.take()) else {
            return;
        };
        cx.spawn(async move |_: &mut AsyncApp| {
            while let Some(job) = inbox.recv().await {
                let parent = parent.lock().ok().and_then(|slot| *slot);
                PAGES.with(|pages| job(&mut pages.borrow_mut(), parent.as_ref()));
            }
        })
        .detach();
    }
}

impl PageThread for UiThread {
    fn post(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}
