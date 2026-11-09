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

pub const MAIN_WINDOW: WindowSpec = WindowSpec {
    title: "Quiver",
    width: 1200,
    height: 800,
    min_width: 800,
    min_height: 600,
    resizable: true,
    background_throttling_disabled: true,
    macos_overlay_titlebar: true,
    macos_traffic_lights: TrafficLights { x: 12, y: 20 },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_tauri_conf_json() {
        let conf = include_str!("../../../src-tauri/tauri.conf.json");
        for needle in [
            format!("\"width\": {}", MAIN_WINDOW.width),
            format!("\"height\": {}", MAIN_WINDOW.height),
            format!("\"minWidth\": {}", MAIN_WINDOW.min_width),
            format!("\"minHeight\": {}", MAIN_WINDOW.min_height),
            format!("\"title\": \"{}\"", MAIN_WINDOW.title),
        ] {
            assert!(conf.contains(&needle), "{needle}");
        }
    }
}
