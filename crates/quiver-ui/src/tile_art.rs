//! A tile's art area: the banner (hue-tinted gradient) or the drawn plain tile.
//!
//! Proportions are fractions of the tile width, straight from `TileAfter`.

use gpui::prelude::*;
use gpui::{
    div, hsla, linear_color_stop, linear_gradient, px, white, App, FontWeight, SharedString,
};
use quiver_theme::FontFamily;

use crate::icon::{icon_with_stroke, Glyph};
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileArtKind {
    /// A banner tile: a dark tinted field with stripes, a circle and the arrow mark.
    Banner { hue: u16 },
    /// No banner: ghost initial, icon, title and publisher drawn on the tile.
    Drawn {
        initial: SharedString,
        name: SharedString,
        publisher: SharedString,
        hue: u16,
    },
}

fn hue(
    h: u16,
    s: f32,
    l: f32,
) -> gpui::Hsla {
    hsla(f32::from(h % 360) / 360.0, s, l, 1.0)
}

/// The art's children (the container is built by the tile, which owns size, radius and clipping).
pub(crate) fn art_children(
    kind: &TileArtKind,
    w: f32,
    cx: &App,
) -> Vec<gpui::AnyElement> {
    match kind {
        TileArtKind::Banner { hue: h } => banner(*h, w),
        TileArtKind::Drawn {
            initial,
            name,
            publisher,
            hue: h,
        } => drawn(initial, name, publisher, *h, w, cx),
    }
}

fn lift(h: u16) -> f32 {
    if h >= 360 {
        0.09
    } else {
        0.0
    }
}

pub(crate) fn banner_background(h: u16) -> gpui::Hsla {
    hue(h, 0.40, 0.17 + lift(h))
}

/// The banner as a free-standing block (a collection header), `w` wide and half as tall.
pub fn banner_block(
    h: u16,
    w: f32,
) -> gpui::Div {
    div()
        .relative()
        .overflow_hidden()
        .rounded(px(12.))
        .w(px(w))
        .h(px(w / 2.0))
        .flex_none()
        .bg(banner_background(h))
        .children(banner(h, w))
}

fn banner(
    h: u16,
    w: f32,
) -> Vec<gpui::AnyElement> {
    let mut out = Vec::new();
    let period = w * 0.1;
    let mut x = 0.0;
    while x < w {
        out.push(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(x))
                .w(px((w * 0.00167).max(1.0)))
                .bg(hue(h, 0.40, 0.24 + lift(h)))
                .into_any_element(),
        );
        x += period;
    }
    out.push(
        div()
            .absolute()
            .rounded_full()
            .size(px(w * 0.4333))
            .left(px(w * 0.5833))
            .top(px(-w * 0.1167))
            .bg(hue(h, 0.46, 0.23 + lift(h)))
            .into_any_element(),
    );
    let glyph = w * 0.4267;
    out.push(
        div()
            .absolute()
            .left(px(w * 0.2867))
            .top(px(w * 0.0733))
            .child(icon_with_stroke(Glyph::Mark, glyph, white(), 1.6))
            .into_any_element(),
    );
    out
}

fn drawn(
    initial: &SharedString,
    name: &SharedString,
    publisher: &SharedString,
    h: u16,
    w: f32,
    cx: &App,
) -> Vec<gpui::AnyElement> {
    let p = cx.quiver().palette;
    let pad = w * 0.0625;
    let icon = w * 0.105;
    let letter = div()
        .absolute()
        .right(px(-w * 0.015))
        .bottom(px(-w * 0.17))
        .text_size(px(w * 0.58))
        .line_height(px(w * 0.58))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_color(p.foreground.opacity(0.048))
        .child(initial.clone());
    let mark = div()
        .absolute()
        .left(px(pad))
        .top(px(pad))
        .size(px(icon))
        .rounded(px(icon * 0.24))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.52, 0.38 + lift(h)), 0.),
            linear_color_stop(hue(h, 0.48, 0.30 + lift(h)), 1.),
        ))
        .child(icon_with_stroke(Glyph::Mark, icon * 0.6, white(), 1.9));
    let title = div()
        .absolute()
        .left(px(pad))
        .right(px(pad))
        .bottom(px(pad))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(w * 0.086))
                .line_height(px(w * 0.086 * 1.15))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.foreground.opacity(0.88))
                .truncate()
                .child(name.clone()),
        )
        .child(
            div()
                .mt(px(w * 0.013))
                .min_h(px(w * 0.05))
                .font_family(FontFamily::Mono.family())
                .text_size(px(w * 0.0405))
                .line_height(px(w * 0.0405 * 1.2))
                .text_color(p.foreground.opacity(0.33))
                .truncate()
                .child(publisher.to_uppercase()),
        );
    vec![
        letter.into_any_element(),
        mark.into_any_element(),
        title.into_any_element(),
    ]
}
