//! The main window as seen by the webviews that live inside it.

use std::sync::Arc;

use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};

/// The main window's native handle. It is only ever turned into a [`WindowHandle`] on the thread that creates
/// a webview (the GTK thread on Linux, the UI thread elsewhere), while the main window is open: the services
/// shut every page down before the window goes.
#[derive(Clone)]
pub struct ParentHandle(Arc<Source>);

enum Source {
    #[cfg(not(target_os = "linux"))]
    Fixed(RawWindowHandle),
    /// Linux: GPUI's X11 windows cannot report their handle (it panics), so the window is found among this
    /// process's X windows (see [`super::x11_lookup`]) the first time a page needs it, on the page thread:
    /// by then the window is mapped, and no X round trip delays the first frame.
    #[cfg(target_os = "linux")]
    ThisProcess(std::sync::OnceLock<Option<RawWindowHandle>>),
}

// SAFETY: the value is plain data (a window id or a pointer); it is dereferenced only by the thread
// that owns the page, as the type's docs say.
unsafe impl Send for Source {}
unsafe impl Sync for Source {}

impl ParentHandle {
    #[cfg(target_os = "linux")]
    pub fn of(_: &impl HasWindowHandle) -> Option<Self> {
        Some(Self(Arc::new(Source::ThisProcess(Default::default()))))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn of(window: &impl HasWindowHandle) -> Option<Self> {
        Some(Self(Arc::new(Source::Fixed(
            window.window_handle().ok()?.as_raw(),
        ))))
    }

    fn raw(&self) -> Option<RawWindowHandle> {
        match &*self.0 {
            #[cfg(not(target_os = "linux"))]
            Source::Fixed(raw) => Some(*raw),
            #[cfg(target_os = "linux")]
            Source::ThisProcess(found) => *found.get_or_init(|| {
                let id = super::x11_lookup::main_window()?;
                // wry's X11 backend wants an Xlib handle.
                Some(RawWindowHandle::Xlib(
                    raw_window_handle::XlibWindowHandle::new(id),
                ))
            }),
        }
    }
}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = self.raw().ok_or(HandleError::Unavailable)?;
        // SAFETY: see the type's docs; the window outlives every page.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}
