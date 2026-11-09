//! The UI fonts, embedded so the app looks the same on every machine: Inter (UI) and JetBrains Mono
//! (versions, namespaces, console). Both are SIL OFL; the licences sit next to the files.
//!
//! Static cuts rather than the variable files: GPUI's text system picks a weight by face, and the
//! board's weights (400, 500, 600, 700, 800) map one to one onto these.

use std::borrow::Cow;

use gpui::App;

const FILES: [&[u8]; 7] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Inter-Bold.ttf"),
    include_bytes!("../assets/fonts/Inter-ExtraBold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf"),
];

/// Registers the fonts with the text system. Call before opening the first window.
pub fn load(cx: &mut App) {
    quiver_ui::load_fonts(cx, FILES.iter().map(|f| Cow::Borrowed(*f)).collect());
}
