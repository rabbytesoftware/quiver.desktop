//! The header's art: the banner at the right and the arrow's app icon.

use gpui::prelude::*;
use gpui::{
    div, hsla, img, linear_color_stop, linear_gradient, px, white, IntoElement, ObjectFit,
    StyledImage,
};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon_with_stroke, Glyph};

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
/// it on a narrow window, the content's width).
pub(super) fn banner_art(
    h: Art,
    w: f32,
    image: Option<&str>,
    muted: gpui::Hsla,
) -> gpui::AnyElement {
    let probe_it = |el: gpui::Div| el.child(probe("arrow.banner", Kind::Other, "banner", true, ""));
    let Some(url) = image.filter(|u| !u.is_empty()) else {
        return probe_it(drawn_banner(h, w)).into_any_element();
    };
    // Old hero: `bg-muted bg-contain bg-center` on the 2:1 box. Muted while it loads; the drawn banner if it
    // cannot load.
    let url = gpui::SharedString::from(url.to_owned());
    probe_it(
        div()
            .relative()
            .flex_none()
            .w(px(w))
            .h(px(w * HEIGHT / BANNER_W))
            .rounded(px(12.))
            .bg(muted)
            .child(
                img(url)
                    .size_full()
                    .rounded(px(12.))
                    .object_fit(ObjectFit::Contain)
                    .with_fallback(move || drawn_banner(h, w).into_any_element()),
            ),
    )
    .into_any_element()
}

fn drawn_banner(
    h: Art,
    w: f32,
) -> gpui::Div {
    let height = w * HEIGHT / BANNER_W;
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
        .h(px(height))
        .overflow_hidden()
        .rounded(px(12.))
        .bg(hue(h, 0.40, 17.0))
        .children(stripes)
        .child({
            // Cut by the banner's rounded corner: GPUI's clip is rectangular.
            let r = w * 260.0 / 1200.0;
            quiver_ui::clipped_disc(
                (w * 700.0 / 1200.0 + r, w * -140.0 / 1200.0 + r, r),
                (w, height, 12.),
                hue(h, 0.46, 23.0),
            )
        })
        .child(
            div()
                .absolute()
                .left(px(w * 149.6 / BANNER_W))
                .top(px(w * 38.6 / BANNER_W))
                .child(icon_with_stroke(Glyph::Mark, w * 0.32, white(), 1.6)),
        )
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
    let photo = image.filter(|u| !u.is_empty()).map(|u| {
        quiver_ui::photo(u.to_owned().into(), ObjectFit::Cover).rounded(px(size * 10. / 44.))
    });
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .rounded(px(size * 10. / 44.))
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
        ))
        .children(photo)
}
