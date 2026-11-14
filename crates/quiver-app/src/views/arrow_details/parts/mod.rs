//! The sections of the arrow page, one file each.

mod action_row;
mod banner;
mod body;
mod details;
mod dialogs;
mod header;
pub(super) mod icons;
mod popovers;
mod selector;
mod states;

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, Window};

pub(super) use dialogs::dialogs;
pub(super) use selector::switch_dialog;
pub(super) use states::{load_error, not_found, skeleton};

use super::page::Page;
use super::ArrowDetailsView;

/// Below this content width the Details card goes under the body (the old page's `RAIL_MIN_WIDTH`).
const RAIL_MIN_WIDTH: f32 = 860.;
const PAD_LEFT: f32 = 44.;
const PAD_RIGHT: f32 = 60.;

pub(super) use crate::views::hero_fit::Fit;

/// Header, then README (or the glance) beside the Details card.
pub(super) fn page(
    view: &mut ArrowDetailsView,
    page: &Page,
    window: &mut Window,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let viewport = f32::from(window.viewport_size().width);
    let sidebar = view.app.read(cx).shell.sidebar_width as f32;
    let fit = Fit::at(viewport, sidebar, PAD_LEFT + PAD_RIGHT);
    // The old page's rule (`RAIL_MIN_WIDTH`), measured as it did: the main area inside its `px-11` (44px each
    // side). Narrower, the Details card goes under the body.
    let beside = viewport - sidebar - 2. * 44. >= RAIL_MIN_WIDTH;
    let body = body::body(view, page, window, cx);
    let details = details::details(view, page, cx);
    let row = if beside {
        div()
            .flex()
            .items_start()
            .gap(px(60.))
            .child(body)
            .children(details)
    } else {
        // Stacked, every block gets the width outright: the README's lists and code blocks size against it.
        div()
            .flex()
            .flex_col()
            .gap(px(40.))
            .child(div().flex().w(px(fit.content.min(560.))).child(body))
            .children(details)
    };
    div()
        .w_full()
        .pl(px(PAD_LEFT))
        .pr(px(PAD_RIGHT))
        .pt(px(25.))
        .pb(px(48.))
        .child(header::header(view, page, fit, cx))
        .child(row.mt(px(41.)))
}
