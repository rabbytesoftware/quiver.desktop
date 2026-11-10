//! Which display server the window talks to.

/// Arrow apps are webviews embedded in the main window, and wry can only embed into an X11 window. On a
/// Wayland session the app therefore runs through XWayland (the way the Tauri app's GTK child views
/// did). Call it first thing in `main`, before any thread exists.
pub fn prefer_x11() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }
}
