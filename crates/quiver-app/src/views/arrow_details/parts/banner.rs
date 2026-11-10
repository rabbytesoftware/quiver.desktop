//! The header's art: the banner at the right and the arrow's app icon.

use gpui::prelude::*;
use gpui::{div, hsla, linear_color_stop, linear_gradient, px, white, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon_with_stroke, Glyph};

const BANNER_W: f32 = 440.0;
pub(super) const HEIGHT: f32 = 218.0;

#[derive(Clone, Copy)]
pub(super) struct Art {
    pub hue: u16,
    pub lift: f32,
}

impl Art {
    /// The demo world's stand-in art: a hue quantised to 24 steps and a two-level lift of the namespace's hash.
    pub fn mock(namespace: &str) -> Self {
        let mut hash: u32 = 5381;
        for byte in namespace.bytes() {
            hash = hash.wrapping_mul(33) ^ u32::from(byte);
        }
        hash = (hash ^ (hash >> 16)).wrapping_mul(0x7feb_352d);
        hash = (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b);
        hash ^= hash >> 16;
        Self {
            hue: (hash % 24) as u16 * 15,
            lift: ((hash >> 8) % 2) as f32 * 9.0,
        }
    }

    pub fn hashed(namespace: &str) -> Self {
        Self {
            hue: quiver_state::tile::hue_for(namespace),
            lift: 0.0,
        }
    }
}

fn hue(
    art: Art,
    s: f32,
    l: f32,
) -> gpui::Hsla {
    hsla(
        f32::from(art.hue % 360) / 360.0,
        s,
        (l + art.lift) / 100.0,
        1.0,
    )
}

pub(super) fn banner_art(h: Art) -> impl IntoElement {
    let w = BANNER_W;
    let period = w * 0.1;
    let stripes = (1..11).map(move |i| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(period * i as f32 - w / 1200.0))
            .w(px(w / 600.0))
            .bg(hue(h, 0.40, 24.0))
    });
    div()
        .relative()
        .flex_none()
        .w(px(w))
        .h(px(HEIGHT))
        .overflow_hidden()
        .rounded(px(12.))
        .bg(hue(h, 0.40, 17.0))
        .children(stripes)
        .child(
            div()
                .absolute()
                .rounded_full()
                .size(px(w * 520.0 / 1200.0))
                .left(px(w * 700.0 / 1200.0))
                .top(px(w * -140.0 / 1200.0))
                .bg(hue(h, 0.46, 23.0)),
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

pub(super) fn app_icon(h: Art) -> impl IntoElement {
    div()
        .flex_none()
        .size(px(44.))
        .rounded(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.52, 38.0), 0.),
            linear_color_stop(hue(h, 0.48, 30.0), 1.),
        ))
        .child(icon_with_stroke(Glyph::Mark, 27., white(), 1.9))
}
