//! Linux: wry's X11 backend needs GTK, and GPUI is not GTK. GTK therefore gets a thread of its own, which
//! creates every webview and runs the GTK main loop; the main window is its X11 parent. Jobs reach the
//! thread through a GLib channel.

use std::cell::RefCell;
use std::sync::Mutex;

use gtk::glib;

use super::pages::Pages;
use super::{Job, PageThread, ParentSlot};

thread_local! {
    static PAGES: RefCell<Pages> = RefCell::new(Pages::default());
}

pub(super) struct GtkThread {
    jobs: Mutex<glib::Sender<Job>>,
}

impl GtkThread {
    pub fn start(parent: ParentSlot) -> Result<Self, String> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("quiver-gtk".into())
            .spawn(move || {
                if let Err(e) = gtk::init() {
                    let _ = ready_tx.send(Err(format!("GTK did not start: {e}")));
                    return;
                }
                // glib 0.18 (the version wry's GTK is on) has no non-deprecated way to hand a closure to
                // a GTK thread from another thread.
                #[allow(deprecated)]
                let (tx, rx) = glib::MainContext::channel::<Job>(glib::Priority::DEFAULT);
                rx.attach(None, move |job| {
                    let parent = parent.lock().ok().and_then(|slot| slot.clone());
                    PAGES.with(|pages| job(&mut pages.borrow_mut(), parent.as_ref()));
                    glib::ControlFlow::Continue
                });
                let _ = ready_tx.send(Ok(tx));
                gtk::main();
            })
            .map_err(|e| e.to_string())?;
        let jobs = ready_rx.recv().map_err(|e| e.to_string())??;
        Ok(Self {
            jobs: Mutex::new(jobs),
        })
    }
}

impl PageThread for GtkThread {
    fn post(
        &self,
        job: Job,
    ) {
        if let Ok(jobs) = self.jobs.lock() {
            let _ = jobs.send(job);
        }
    }
}
