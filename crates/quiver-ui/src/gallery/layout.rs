//! Section and row scaffolding for the gallery pages.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, FontWeight, SharedString};

use crate::theme::ActiveQuiver;

/// A titled block of rows.
pub fn section(title: &'static str, rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let p = cx.quiver().palette;
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .pb(px(18.))
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.foreground)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}

/// A captioned horizontal row of demo items.
pub fn row(caption: impl Into<SharedString>, items: Vec<AnyElement>, cx: &App) -> AnyElement {
    let p = cx.quiver().palette;
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(
            div()
                .w(px(110.))
                .flex_none()
                .text_size(px(11.))
                .text_color(p.muted_foreground)
                .child(caption.into()),
        )
        .children(items)
        .into_any_element()
}

/// Gallery ids are built at runtime; GPUI wants an `ElementId`.
pub fn gid(name: String) -> gpui::ElementId {
    gpui::ElementId::Name(name.into())
}
