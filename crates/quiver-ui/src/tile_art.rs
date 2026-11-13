//! A tile's art area: the banner (hue-tinted gradient) or the drawn plain tile.
//!
//! Proportions are fractions of the tile width, straight from `TileAfter`.

use gpui::prelude::*;
use gpui::{
    div, hsla, linear_color_stop, linear_gradient, px, white, App, FontWeight, SharedString,
};
use quiver_theme::FontFamily;

use crate::photo::Photo;
use crate::theme::ActiveQuiver;
use crate::tracked::TrackedText;

/// What a tile's drawn card says: ghost initial, icon, title and publisher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardArt {
    pub initial: SharedString,
    pub name: SharedString,
    pub publisher: SharedString,
    pub hue: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileArtKind {
    /// The arrow has a banner: the card shows only until it has loaded (and for good if it cannot).
    Banner(CardArt),
    /// No banner: the card is the art.
    Drawn(CardArt),
}

impl TileArtKind {
    pub(crate) fn card(&self) -> &CardArt {
        match self {
            Self::Banner(card) | Self::Drawn(card) => card,
        }
    }
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
    card: &CardArt,
    w: f32,
    icon: Option<&SharedString>,
    cx: &App,
) -> Vec<gpui::AnyElement> {
    drawn(
        &card.initial,
        &card.name,
        &card.publisher,
        card.hue,
        w,
        icon,
        cx,
    )
}

/// The card as a layer filling its parent: what a banner's [`Photo`] draws until the banner has loaded.
pub fn card_stand_in(
    card: &CardArt,
    w: f32,
    icon: Option<&SharedString>,
    cx: &App,
) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .debug_selector(|| "tile-card".into())
        .bg(cx.quiver().palette.muted)
        .children(art_children(card, w, icon, cx))
}

fn lift(h: u16) -> f32 {
    if h >= 360 {
        0.09
    } else {
        0.0
    }
}

/// A free-standing banner block showing `url`, the card until it has loaded.
pub fn banner_photo(
    url: impl Into<SharedString>,
    card: &CardArt,
    w: f32,
    icon: Option<&SharedString>,
    cx: &App,
) -> gpui::Div {
    div()
        .relative()
        .overflow_hidden()
        .rounded(px(12.))
        .w(px(w))
        .h(px(w / 2.0))
        .flex_none()
        .child(
            Photo::new(url.into())
                .radius(12.)
                .stand_in(card_stand_in(card, w, icon, cx)),
        )
}

/// The default icon of an arrow with none of its own (or none loaded yet): its initial, white, on its hue's
/// gradient. Fills its (relatively positioned) parent; `size` is the parent's width, `radius` its corners.
pub fn monogram(
    h: u16,
    size: f32,
    radius: f32,
    initial: &SharedString,
) -> gpui::Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(radius))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(size * 0.52))
        .line_height(px(size * 0.52))
        .font_weight(FontWeight::BOLD)
        .text_color(white())
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.52, 0.38 + lift(h)), 0.),
            linear_color_stop(hue(h, 0.48, 0.30 + lift(h)), 1.),
        ))
        .child(initial.clone())
}

/// The arrow's icon, `size` wide: its own image once it has loaded, the [`monogram`] until then (and for good
/// when there is none or it cannot load).
pub(crate) fn mark(
    h: u16,
    size: f32,
    image: Option<&SharedString>,
    initial: &SharedString,
) -> gpui::Div {
    let radius = size * 0.24;
    let drawn = || monogram(h, size, radius, initial);
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .child(match image {
            Some(url) => Photo::new(url.clone())
                .radius(radius)
                .stand_in(drawn())
                .into_any_element(),
            None => drawn().into_any_element(),
        })
}

fn drawn(
    initial: &SharedString,
    name: &SharedString,
    publisher: &SharedString,
    h: u16,
    w: f32,
    image: Option<&SharedString>,
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
    let mark = mark(h, icon, image, initial)
        .absolute()
        .left(px(pad))
        .top(px(pad));
    let title = div()
        .absolute()
        .left(px(pad))
        .right(px(pad))
        .bottom(px(pad))
        .flex()
        .flex_col()
        .child(
            TrackedText::new(
                name.clone(),
                w * 0.086,
                w * 0.086 * 1.15,
                -0.03 * w * 0.086,
                p.foreground.opacity(0.88),
            )
            .weight(FontWeight::SEMIBOLD),
        )
        .child(
            div().mt(px(w * 0.013)).min_h(px(w * 0.05)).child(
                TrackedText::new(
                    publisher.to_uppercase(),
                    w * 0.0405,
                    w * 0.0405 * 1.2,
                    0.05 * w * 0.0405,
                    p.foreground.opacity(0.33),
                )
                .family(FontFamily::Mono.family()),
            ),
        );
    vec![
        letter.into_any_element(),
        mark.into_any_element(),
        title.into_any_element(),
    ]
}
