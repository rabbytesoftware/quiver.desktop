//! The header's app icon (the banner art is `views::hero_fit`, shared with the collection page).

use gpui::prelude::*;
use gpui::{div, px, IntoElement};
use quiver_ui::CardArt;

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
