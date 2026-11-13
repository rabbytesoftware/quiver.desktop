//! Linux: wry's X11 backend needs GTK, and GPUI is not GTK. Every webview is created on the process's GTK
//! thread ([`crate::services::gtk_thread`]), with the main window as its X11 parent.

use std::cell::RefCell;

use super::pages::Pages;
use super::{Job, PageThread, ParentSlot};
use crate::services::gtk_thread;

thread_local! {
    static PAGES: RefCell<Pages> = RefCell::new(Pages::default());
}

/// Runs `f` on the pages, from a GTK callback (not from inside a job, where they are already borrowed).
pub(super) fn with_pages(f: impl FnOnce(&mut Pages)) {
    PAGES.with(|pages| {
        if let Ok(mut pages) = pages.try_borrow_mut() {
            f(&mut pages);
        }
    });
}

pub(super) struct GtkThread {
    parent: ParentSlot,
}

impl GtkThread {
    pub fn start(parent: ParentSlot) -> Result<Self, String> {
        gtk_thread::ensure()?;
        Ok(Self { parent })
    }
}

impl PageThread for GtkThread {
    fn post(
        &self,
        job: Job,
    ) {
        let parent = self.parent.clone();
        let posted = gtk_thread::post(Box::new(move || {
            let parent = parent.lock().ok().and_then(|slot| slot.clone());
            PAGES.with(|pages| job(&mut pages.borrow_mut(), parent.as_ref()));
        }));
        if let Err(e) = posted {
            log::error!("the GTK thread is gone: {e}");
        }
    }
}
