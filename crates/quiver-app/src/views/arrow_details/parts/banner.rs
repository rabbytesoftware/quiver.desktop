//! The header's art: the banner at the right and the arrow's app icon.

use gpui::prelude::*;
use gpui::{div, hsla, linear_color_stop, linear_gradient, px, white, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon_with_stroke, Glyph};

const BANNER_W: f32 = 440.0;
pub(super) const HEIGHT: f32 = 218.0;

fn hue(
    h: u16,
    s: f32,
    l: f32,
) -> gpui::Hsla {
    hsla(f32::from(h % 360) / 360.0, s, l, 1.0)
}

pub(super) fn banner_art(h: u16) -> impl IntoElement {
    let w = BANNER_W;
    let period = w * 0.08;
    let stripes = (1..12).map(move |i| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(period * i as f32 - w * 0.002))
            .w(px(w * 0.002))
            .bg(white().opacity(0.07))
    });
    div()
        .relative()
        .flex_none()
        .w(px(w))
        .h(px(HEIGHT))
        .overflow_hidden()
        .rounded(px(12.))
        .bg(hue(h, 0.55, 0.24))
        .children(stripes)
        .child(
            div()
                .absolute()
                .rounded_full()
                .size(px(w * 0.5))
                .right(px(-w * 0.04))
                .top(px(-w * 0.2))
                .bg(hue(h, 0.55, 0.32)),
        )
        .child(
            div()
                .absolute()
                .left(px(149.6))
                .top(px(38.6))
                .child(icon_with_stroke(Glyph::Mark, w * 0.32, white(), 1.6)),
        )
        .child(probe("arrow.banner", Kind::Other, "banner", true, ""))
}

pub(super) fn app_icon(h: u16) -> impl IntoElement {
    div()
        .flex_none()
        .size(px(44.))
        .rounded(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.48, 0.50), 0.),
            linear_color_stop(hue(h, 0.52, 0.33), 1.),
        ))
        .child(icon_with_stroke(Glyph::Mark, 27., white(), 1.9))
}
