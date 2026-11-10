//! The sections of the arrow page, one file each.

mod action_row;
mod body;
mod details;
mod dialogs;
mod header;
pub(super) mod icons;
mod popovers;
mod states;

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, Window};

pub(super) use dialogs::dialogs;
pub(super) use states::{load_error, not_found, skeleton};

use super::page::Page;
use super::ArrowDetailsView;

/// Header, then README (or the glance) beside the Details card.
pub(super) fn page(
    view: &mut ArrowDetailsView,
    page: &Page,
    window: &mut Window,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    div()
        .w_full()
        .pl(px(44.))
        .pr(px(60.))
        .pt(px(25.))
        .pb(px(48.))
        .child(header::header(view, page, cx))
        .child(
            div()
                .mt(px(41.))
                .flex()
                .items_start()
                .gap(px(60.))
                .child(body::body(view, page, window, cx))
                .child(details::details(view, page, cx)),
        )
}
