//! The banners on top of the engine tab: restart needed, settings Core corrected, a failed patch.

use gpui::prelude::*;
use gpui::{div, px, IntoElement, SharedString};
use quiver_ui::Palette;

fn notice(
    p: &Palette,
    text: SharedString,
    destructive: bool,
) -> impl IntoElement {
    div()
        .mx(px(4.))
        .mb(px(12.))
        .px(px(10.))
        .py(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(if destructive {
            p.destructive.opacity(0.4)
        } else {
            p.border
        })
        .bg(if destructive {
            p.destructive.opacity(0.1)
        } else {
            p.muted.opacity(0.45)
        })
        .text_size(px(12.))
        .line_height(px(19.5))
        .text_color(if destructive {
            p.destructive
        } else {
            p.muted_foreground
        })
        .child(text)
}

pub fn banner(
    p: &Palette,
    text: impl Into<SharedString>,
) -> impl IntoElement {
    notice(p, text.into(), false)
}

pub fn failure(
    p: &Palette,
    text: impl Into<SharedString>,
) -> impl IntoElement {
    notice(p, text.into(), true)
}
