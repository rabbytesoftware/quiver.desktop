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

pub(crate) fn banner_background(h: u16) -> gpui::Hsla {
    hue(h, 0.55, 0.24)
}

fn banner(
    h: u16,
    w: f32,
) -> Vec<gpui::AnyElement> {
    let mut out = Vec::new();
    let period = w * 0.08;
    let mut x = period - w * 0.002;
    while x < w {
        out.push(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(x))
                .w(px((w * 0.002).max(1.0)))
                .bg(white().opacity(0.07))
                .into_any_element(),
        );
        x += period;
    }
    out.push(
        div()
            .absolute()
            .rounded_full()
            .size(px(w * 0.5))
            .right(px(-w * 0.04))
            .top(px(-w * 0.2))
            .bg(hue(h, 0.55, 0.32))
            .into_any_element(),
    );
    let glyph = w * 0.32;
    out.push(
        div()
            .absolute()
            .left(px(w * 0.5 - glyph / 2.0))
            .top(px(w * 0.25 - glyph / 2.0))
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
        .right(px(-w * 0.005))
        .bottom(px(-w * 0.125))
        .text_size(px(w * 0.58))
        .line_height(px(w * 0.58))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_color(p.border)
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
            linear_color_stop(hue(h, 0.48, 0.50), 0.),
            linear_color_stop(hue(h, 0.52, 0.33), 1.),
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
                .text_size(px(w * 0.0845))
                .line_height(px(w * 0.0845 * 1.1))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.foreground)
                .truncate()
                .child(name.clone()),
        )
        .child(
            div()
                .mt(px(w * 0.012))
                .min_h(px(w * 0.05))
                .font_family(FontFamily::Mono.family())
                .text_size(px(w * 0.0405))
                .line_height(px(w * 0.0405 * 1.2))
                .text_color(p.muted_foreground)
                .truncate()
                .child(publisher.to_uppercase()),
        );
    vec![
        letter.into_any_element(),
        mark.into_any_element(),
        title.into_any_element(),
    ]
}
