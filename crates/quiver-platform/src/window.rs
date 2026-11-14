//! The main window as `tauri.conf.json` (and `tauri.macos.conf.json`) declared it.

/// Where the traffic-light buttons sit on the macOS overlay title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrafficLights {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSpec {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub resizable: bool,
    /// `backgroundThrottling: "disabled"`: the webview kept running while
    /// hidden or occluded. For a native window this means keeping live event
    /// streams rendering, not pausing them.
    pub background_throttling_disabled: bool,
    /// macOS only: `titleBarStyle: "Overlay"` + `hiddenTitle`, content under a
    /// transparent title bar with the title text hidden.
    pub macos_overlay_titlebar: bool,
    pub macos_traffic_lights: TrafficLights,
}

/// The height of the strip the traffic lights occupy at the top-left of an overlay-title-bar window (the top
/// row of the sidebar, which also holds the back and forward buttons).
pub const TITLEBAR_ROW: f32 = 34.;

impl WindowSpec {
    /// The window draws under a transparent title bar (macOS only), so what sits at its top-left must leave
    /// room for the traffic lights.
    pub fn overlay_titlebar(&self) -> bool {
        cfg!(target_os = "macos") && self.macos_overlay_titlebar
    }
}

pub const MAIN_WINDOW: WindowSpec = WindowSpec {
    title: "Quiver",
    width: 1200,
    height: 800,
    min_width: 800,
    min_height: 600,
    resizable: true,
    background_throttling_disabled: true,
    macos_overlay_titlebar: true,
    // `y` is where the buttons' top edge sits below the window top (GPUI), not their centre: the 34px top row's
    // centre is 17, the buttons are 14 tall, so 10.
    macos_traffic_lights: TrafficLights { x: 12, y: 10 },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_the_window_the_tauri_build_declared() {
        // Former tauri.conf.json: 1200x800, minimum 800x600, titled Quiver.
        assert_eq!((MAIN_WINDOW.width, MAIN_WINDOW.height), (1200, 800));
        assert_eq!((MAIN_WINDOW.min_width, MAIN_WINDOW.min_height), (800, 600));
        assert_eq!(MAIN_WINDOW.title, "Quiver");
    }
}
