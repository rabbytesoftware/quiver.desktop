//! The header's art: the banner at the right and the arrow's app icon.

use gpui::prelude::*;
use gpui::{div, px, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::CardArt;

pub(super) const BANNER_W: f32 = 440.0;
pub(super) const HEIGHT: f32 = 218.0;

/// The banner art `w` wide, at the 440×218 board's proportions (beside the header's text, `BANNER_W`; above
/// it on a narrow window, the content's width). The arrow's own banner covers it once loaded; until then, and
/// when the arrow has none or it cannot load, the drawn card shows.
pub(super) fn banner_art(
    card: &CardArt,
    w: f32,
    image: Option<&str>,
    icon: Option<&gpui::SharedString>,
    cx: &gpui::App,
) -> gpui::AnyElement {
    let height = w * HEIGHT / BANNER_W;
    let probe_it = |el: gpui::Div| el.child(probe("arrow.banner", Kind::Other, "banner", true, ""));
    match image.filter(|u| !u.is_empty()) {
        Some(url) => {
            probe_it(quiver_ui::banner_photo(url.to_owned(), card, w, icon, cx).h(px(height)))
        }
        None => probe_it(
            div()
                .relative()
                .flex_none()
                .w(px(w))
                .h(px(height))
                .overflow_hidden()
                .rounded(px(12.))
                .child(quiver_ui::card_stand_in(card, w, icon, cx)),
        ),
    }
    .into_any_element()
}

pub(super) fn app_icon(
    card: &CardArt,
    image: Option<&str>,
) -> impl IntoElement {
    app_icon_sized(card, 44., image)
}

/// The arrow's `media.icon` once it has loaded; until then, or if it cannot, its monogram.
pub(super) fn app_icon_sized(
    card: &CardArt,
    size: f32,
    image: Option<&str>,
) -> impl IntoElement {
    let radius = size * 10. / 44.;
    let drawn = quiver_ui::monogram(card.hue, size, radius, &card.initial);
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .rounded(px(radius))
        .child(match image.filter(|u| !u.is_empty()) {
            Some(u) => quiver_ui::Photo::new(u.to_owned())
                .radius(radius)
                .stand_in(drawn)
                .into_any_element(),
            None => drawn.into_any_element(),
        })
}
