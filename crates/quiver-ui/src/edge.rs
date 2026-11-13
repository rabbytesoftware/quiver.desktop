//! Coss's inner edges. Every control draws a 1px line just inside (or just under) its border: a
//! `before:` pseudo-element with `shadow-[0_1px_black/4%]` in light mode and `shadow-[0_-1px_white/6%]` in
//! dark mode, and the filled variants an `inset-shadow-[0_1px_white/16%]` highlight along the top. GPUI has
//! no inset shadows and a shadow under a transparent element would tint its whole area, so each edge is a
//! rounded, non-interactive overlay with a one-sided 1px border: the same crescent that tapers at the corners.

use gpui::prelude::*;
use gpui::{div, px, Div, Hsla};

use crate::theme::QuiverTheme;

fn overlay(radius: f32) -> Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .rounded(px((radius - 1.0).max(0.0)))
}

/// `inset-shadow-[0_1px_<color>]`: a highlight on the top row inside the border.
pub fn inner_highlight(
    radius: f32,
    color: Hsla,
) -> Div {
    overlay(radius)
        .top_0()
        .bottom_0()
        .border_t_1()
        .border_color(color)
}

/// `before:shadow-[0_1px_<color>]`: a line on the border row under the control.
pub fn lower_edge(
    radius: f32,
    color: Hsla,
) -> Div {
    overlay(radius)
        .top_0()
        .bottom(px(-1.0))
        .border_b_1()
        .border_color(color)
}

/// `before:shadow-[0_-1px_<color>]`: a line on the border row above the control.
pub fn upper_edge(
    radius: f32,
    color: Hsla,
) -> Div {
    overlay(radius)
        .top(px(-1.0))
        .bottom_0()
        .border_t_1()
        .border_color(color)
}

/// The edge every Coss surface carries: black 4% under it in light mode, white `dark_alpha` above it in dark
/// mode (6% for controls, 8% for the larger panels).
pub fn control_edge(
    q: &QuiverTheme,
    radius: f32,
    dark_alpha: f32,
) -> Div {
    if q.is_dark() {
        return upper_edge(radius, gpui::white().opacity(dark_alpha));
    }
    lower_edge(radius, gpui::black().opacity(0.04))
}
