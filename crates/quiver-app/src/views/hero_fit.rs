//! How a page header with banner art lays out at the window's width. Arrow details and the collection page
//! share it, so a banner behaves the same on both: beside the text on a wide window, above it on a narrow one,
//! scaled to the room, never wider than it.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, SharedString};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::CardArt;

/// The banner board: 440 x 218.
pub const BANNER_W: f32 = 440.0;
pub const BANNER_H: f32 = 218.0;
/// Below this window width the banner goes above the text (the old hero's `lg` breakpoint).
const BANNER_BESIDE_MIN_VIEWPORT: f32 = 1024.;
/// Beside the banner, the text column keeps at least this much, or the banner goes above instead.
const MIN_TEXT: f32 = 300.;
/// The gap between the text and the banner beside it.
pub const BANNER_GAP: f32 = 12.;

/// How a header lays out at the window's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    /// The page's content width, inside its padding.
    pub content: f32,
    /// The banner beside the text (wide) or above it.
    pub banner_beside: bool,
}

impl Fit {
    /// `padding` is the page's total horizontal padding, `sidebar` the width the sidebar takes.
    pub fn at(
        viewport: f32,
        sidebar: f32,
        padding: f32,
    ) -> Self {
        let content = (viewport - sidebar - padding).max(0.);
        Self {
            content,
            banner_beside: viewport >= BANNER_BESIDE_MIN_VIEWPORT
                && content >= BANNER_W + BANNER_GAP + MIN_TEXT,
        }
    }
}

/// The banner art `w` wide at the board's proportions. The image covers it once loaded; until then, and when
/// there is none or it cannot load, the drawn card shows. `probe_id` names it for the inspector.
pub fn banner_art(
    probe_id: &'static str,
    card: &CardArt,
    w: f32,
    image: Option<&str>,
    icon: Option<&SharedString>,
    cx: &App,
) -> AnyElement {
    let height = w * BANNER_H / BANNER_W;
    let probe_it = |el: gpui::Div| el.child(probe(probe_id, Kind::Other, "banner", true, ""));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_window_puts_the_banner_beside_the_text() {
        let fit = Fit::at(1600., 245., 104.);
        assert!(fit.banner_beside);
        assert_eq!(fit.content, 1251.);
    }

    #[test]
    fn a_window_under_1024_stacks_the_banner_above() {
        assert!(!Fit::at(1000., 0., 104.).banner_beside);
    }

    /// The collection page lost its banner behind the right edge: wide enough a window, but a sidebar wide
    /// enough to leave the text no room beside 440 px of banner.
    #[test]
    fn a_wide_window_with_a_wide_sidebar_still_stacks_when_the_text_would_be_squeezed() {
        let fit = Fit::at(1208., 440., 64.);
        assert_eq!(fit.content, 704.);
        assert!(
            !fit.banner_beside,
            "704 px cannot hold 440 of banner and 300 of text"
        );
    }

    #[test]
    fn content_never_goes_negative() {
        assert_eq!(Fit::at(100., 245., 104.).content, 0.);
    }
}
