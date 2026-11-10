//! The main window as seen by the webviews that live inside it.

use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};

/// A copy of the main window's native handle. It is only ever turned back into a [`WindowHandle`] on the
/// thread that creates a webview (the GTK thread on Linux, the UI thread elsewhere), while the main
/// window is open: the services shut every page down before the window goes.
#[derive(Clone, Copy)]
pub struct ParentHandle(RawWindowHandle);

// SAFETY: the value is plain data (a window id or a pointer); it is dereferenced only by the thread
// that owns the page, as the type's docs say.
unsafe impl Send for ParentHandle {}
unsafe impl Sync for ParentHandle {}

impl ParentHandle {
    /// The handle wry can parent a webview to. GPUI reports the window to X11 through XCB; wry's X11
    /// backend wants Xlib. The window id is the same either way.
    pub fn of(window: &impl HasWindowHandle) -> Option<Self> {
        let raw = window.window_handle().ok()?.as_raw();
        Some(Self(Self::for_wry(raw)))
    }

    #[cfg(target_os = "linux")]
    fn for_wry(raw: RawWindowHandle) -> RawWindowHandle {
        match raw {
            RawWindowHandle::Xcb(h) => RawWindowHandle::Xlib(
                raw_window_handle::XlibWindowHandle::new(h.window.get().into()),
            ),
            other => other,
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn for_wry(raw: RawWindowHandle) -> RawWindowHandle {
        raw
    }
}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: see the type's docs; the window outlives every page.
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}
