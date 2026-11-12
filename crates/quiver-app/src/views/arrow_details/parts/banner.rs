//! The header's art: the banner at the right and the arrow's app icon.

use gpui::prelude::*;
use gpui::{div, hsla, linear_color_stop, linear_gradient, px, white, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon_with_stroke, CardArt, Glyph};

pub(super) const BANNER_W: f32 = 440.0;
pub(super) const HEIGHT: f32 = 218.0;

#[derive(Clone, Copy)]
pub(super) struct Art {
    pub hue: u16,
    pub lift: f32,
}

impl Art {
    /// The demo world's art: the same swatch (`hue_for` is the old mock world's `paletteOf`).
    pub fn mock(namespace: &str) -> Self {
        Self::hashed(namespace)
    }

    /// The art every other place draws for this arrow (sidebar, tiles): `hue_for`'s hue and its lift bit.
    pub fn hashed(namespace: &str) -> Self {
        let h = quiver_state::tile::hue_for(namespace);
        Self {
            hue: h % 360,
            lift: if h >= 360 { 9.0 } else { 0.0 },
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
    h: Art,
    image: Option<&str>,
) -> impl IntoElement {
    app_icon_sized(h, 44., image)
}

/// The drawn mark, with the arrow's `media.icon` over it once it has loaded (old `ArrowIcon`: the image,
/// the drawn chip when there is none or it fails).
pub(super) fn app_icon_sized(
    h: Art,
    size: f32,
    image: Option<&str>,
) -> impl IntoElement {
    let radius = size * 10. / 44.;
    let drawn = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(radius))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.52, 38.0), 0.),
            linear_color_stop(hue(h, 0.48, 30.0), 1.),
        ))
        .child(icon_with_stroke(
            Glyph::Mark,
            size * 27. / 44.,
            white(),
            1.9,
        ));
    // The arrow's icon replaces the drawn mark once it has loaded; until then, or if it cannot, the mark shows.
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
