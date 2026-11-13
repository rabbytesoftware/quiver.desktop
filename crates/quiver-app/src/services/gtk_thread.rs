//! Linux: the process's one GTK thread. GPUI is not GTK, but two things here are: wry's X11 webviews (the
//! arrow apps' pages) and the native context menu. GTK may only ever run on one thread, so both post their
//! work here; the thread starts on first use and runs the GTK main loop for the rest of the process.

use std::sync::{Mutex, OnceLock};

use gtk::glib;

/// Work for the GTK thread.
pub type GtkJob = Box<dyn FnOnce() + Send>;

static THREAD: OnceLock<Result<Mutex<glib::Sender<GtkJob>>, String>> = OnceLock::new();

/// Starts the thread if it is not running yet; `Err` when GTK could not start (no display).
pub fn ensure() -> Result<(), String> {
    THREAD
        .get_or_init(start)
        .as_ref()
        .map(drop)
        .map_err(Clone::clone)
}

/// Runs `job` on the GTK thread.
pub fn post(job: GtkJob) -> Result<(), String> {
    let jobs = THREAD.get_or_init(start).as_ref().map_err(Clone::clone)?;
    jobs.lock()
        .map_err(|e| e.to_string())?
        .send(job)
        .map_err(|e| e.to_string())
}

fn start() -> Result<Mutex<glib::Sender<GtkJob>>, String> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("quiver-gtk".into())
        .spawn(move || {
            if let Err(e) = gtk::init() {
                let _ = ready_tx.send(Err(format!("GTK did not start: {e}")));
                return;
            }
            // glib 0.18 (the version wry's GTK is on) has no non-deprecated way to hand a closure to a
            // GTK thread from another thread.
            #[allow(deprecated)]
            let (tx, rx) = glib::MainContext::channel::<GtkJob>(glib::Priority::DEFAULT);
            rx.attach(None, move |job| {
                job();
                glib::ControlFlow::Continue
            });
            let _ = ready_tx.send(Ok(tx));
            gtk::main();
        })
        .map_err(|e| e.to_string())?;
    let jobs = ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Mutex::new(jobs))
}
