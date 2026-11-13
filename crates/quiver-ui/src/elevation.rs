//! Tailwind shadow presets (`shadow-xs`, `shadow-md`, `shadow-lg`), tinted with a token colour.

use gpui::{point, px, BoxShadow, Hsla};

fn shadow(
    color: Hsla,
    y: f32,
    blur: f32,
    spread: f32,
) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
    }
}

/// `shadow-xs`: `0 1px 2px 0`.
pub fn shadow_xs(color: Hsla) -> Vec<BoxShadow> {
    vec![shadow(color, 1.0, 2.0, 0.0)]
}

/// `shadow-sm`: `0 1px 3px 0, 0 1px 2px -1px`.
pub fn shadow_sm(color: Hsla) -> Vec<BoxShadow> {
    vec![shadow(color, 1.0, 3.0, 0.0), shadow(color, 1.0, 2.0, -1.0)]
}

/// `shadow-md`: `0 4px 6px -1px, 0 2px 4px -2px`.
pub fn shadow_md(color: Hsla) -> Vec<BoxShadow> {
    vec![shadow(color, 4.0, 6.0, -1.0), shadow(color, 2.0, 4.0, -2.0)]
}

/// `shadow-lg`: `0 10px 15px -3px, 0 4px 6px -4px`.
pub fn shadow_lg(color: Hsla) -> Vec<BoxShadow> {
    vec![
        shadow(color, 10.0, 15.0, -3.0),
        shadow(color, 4.0, 6.0, -4.0),
    ]
}
