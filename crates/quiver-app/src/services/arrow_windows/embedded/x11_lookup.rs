//! Finding the main window on X11.
//!
//! GPUI 0.2.2 does not implement `HasWindowHandle` for its X11 windows (it panics), so the window id wry
//! needs as the pages' parent cannot be asked of GPUI. The window is this process's own, though, and GPUI
//! sets `_NET_WM_PID` on it: the biggest window carrying our pid is the main window.

use std::ffi::{c_long, c_uchar, c_ulong};
use std::os::raw::c_int;

use x11_dl::xlib::{self, Xlib};

/// How deep under the root a window manager's frames can put a client.
const MAX_DEPTH: u8 = 4;

/// An open connection to the X server, closed on drop.
struct Server {
    xlib: Xlib,
    display: *mut xlib::Display,
    pid_atom: c_ulong,
}

impl Server {
    fn open() -> Option<Self> {
        let xlib = Xlib::open().ok()?;
        // SAFETY: opens the default display and looks an atom up on it; both are checked below.
        let (display, pid_atom) = unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return None;
            }
            let atom = (xlib.XInternAtom)(display, c"_NET_WM_PID".as_ptr(), xlib::True);
            (display, atom)
        };
        Some(Self {
            xlib,
            display,
            pid_atom,
        })
    }

    /// `(area, id)` of the biggest window at or under `window` that belongs to `pid`.
    fn biggest(
        &self,
        window: c_ulong,
        pid: u32,
        depth: u8,
    ) -> Option<(u64, c_ulong)> {
        let mut best = None;
        if depth > 0 && self.owner(window) == Some(pid) {
            best = self.area(window).map(|area| (area, window));
        }
        if depth >= MAX_DEPTH {
            return best;
        }
        for child in self.children(window) {
            if let Some(candidate) = self.biggest(child, pid, depth + 1) {
                if best.is_none_or(|(area, _)| candidate.0 > area) {
                    best = Some(candidate);
                }
            }
        }
        best
    }

    fn area(
        &self,
        window: c_ulong,
    ) -> Option<u64> {
        // SAFETY: `XWindowAttributes` is plain data Xlib fills in; the display is open.
        unsafe {
            let mut attributes: xlib::XWindowAttributes = std::mem::zeroed();
            ((self.xlib.XGetWindowAttributes)(self.display, window, &mut attributes) != 0)
                .then(|| u64::try_from(attributes.width * attributes.height).unwrap_or(0))
        }
    }

    fn children(
        &self,
        window: c_ulong,
    ) -> Vec<c_ulong> {
        let (mut root, mut parent) = (0, 0);
        let mut list: *mut c_ulong = std::ptr::null_mut();
        let mut count = 0;
        // SAFETY: Xlib returns `count` window ids at `list`, which is read within that length and freed.
        unsafe {
            if (self.xlib.XQueryTree)(
                self.display,
                window,
                &mut root,
                &mut parent,
                &mut list,
                &mut count,
            ) == 0
                || list.is_null()
            {
                return Vec::new();
            }
            let found = std::slice::from_raw_parts(list, count as usize).to_vec();
            (self.xlib.XFree)(list.cast());
            found
        }
    }

    /// The pid in `window`'s `_NET_WM_PID`, if it has one.
    fn owner(
        &self,
        window: c_ulong,
    ) -> Option<u32> {
        let mut kind: c_ulong = 0;
        let mut format: c_int = 0;
        let (mut items, mut after): (c_ulong, c_ulong) = (0, 0);
        let mut data: *mut c_uchar = std::ptr::null_mut();
        // SAFETY: a one-item CARDINAL property arrives as a C `long` at `data`, read only when Xlib says
        // it is one 32-bit item, then freed.
        unsafe {
            let status = (self.xlib.XGetWindowProperty)(
                self.display,
                window,
                self.pid_atom,
                0,
                1,
                xlib::False,
                xlib::XA_CARDINAL,
                &mut kind,
                &mut format,
                &mut items,
                &mut after,
                &mut data,
            );
            if status != 0 || data.is_null() {
                return None;
            }
            let pid = (items == 1 && format == 32).then(|| *data.cast::<c_long>() as u32);
            (self.xlib.XFree)(data.cast());
            pid
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // SAFETY: the display was opened by `open` and is closed once.
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

/// The id of the biggest window of this process, if the X server shows one.
pub fn main_window() -> Option<c_ulong> {
    let server = Server::open().filter(|s| s.pid_atom != 0)?;
    // SAFETY: the display is open.
    let root = unsafe { (server.xlib.XDefaultRootWindow)(server.display) };
    server
        .biggest(root, std::process::id(), 0)
        .map(|(_, id)| id)
}
