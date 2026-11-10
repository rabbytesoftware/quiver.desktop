//! The banners on top of the engine tab: restart needed, settings Core corrected, a failed patch.

use gpui::prelude::*;
use gpui::{div, px, IntoElement, SharedString};
use quiver_ui::Palette;

fn notice(p: &Palette, text: SharedString, destructive: bool) -> impl IntoElement {
    div()
        .mb(px(16.))
        .px(px(12.))
        .py(px(8.))
        .rounded(px(8.))
        .border_1()
        .border_color(if destructive {
            p.destructive.opacity(0.4)
        } else {
            p.border
        })
        .bg(p.muted)
        .text_size(px(12.5))
        .text_color(if destructive {
            p.destructive
        } else {
            p.muted_foreground
        })
        .child(text)
}

pub fn banner(p: &Palette, text: impl Into<SharedString>) -> impl IntoElement {
    notice(p, text.into(), false)
}

pub fn failure(p: &Palette, text: impl Into<SharedString>) -> impl IntoElement {
    notice(p, text.into(), true)
}
