//! What the page shows before it has an arrow: the loading skeleton, a failed read, and 404.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::i18n::en::arrow;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::arrow_details::ArrowDetailsView;

pub fn skeleton(cx: &mut Context<ArrowDetailsView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let bar = |w: f32, h: f32| div().w(px(w)).h(px(h)).rounded(px(5.)).bg(p.muted);
    div()
        .relative()
        .w_full()
        .pl(px(44.))
        .pr(px(60.))
        .pt(px(25.))
        .flex()
        .gap(px(12.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .gap(px(16.))
                        .child(div().size(px(44.)).rounded(px(10.)).bg(p.muted))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(bar(180., 20.))
                                .child(bar(220., 12.)),
                        ),
                )
                .child(bar(120., 18.))
                .child(bar(360., 16.))
                .child(div().mt(px(40.)).child(bar(250., 40.))),
        )
        .child(div().w(px(440.)).h(px(218.)).rounded(px(12.)).bg(p.muted))
        .child(probe("arrow.skeleton", Kind::Other, "Loading", true, ""))
}

fn notice(
    id: &'static str,
    title: String,
    detail: String,
    cx: &Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .p(px(24.))
        .text_size(px(14.))
        .text_color(p.muted_foreground)
        .child(title)
        .child(probe(id, Kind::Other, id, true, detail))
}

/// The daemon could not be read, or answered with an error.
pub fn load_error(
    view: &mut ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let title = view.app.read(cx).t(arrow::ERROR);
    let body = view.load.read(|l| l.error.clone()).unwrap_or_default();
    notice("arrow.error", title, body, cx)
}

/// The daemon (or the library) has no arrow at this namespace.
pub fn not_found(
    view: &mut ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let title = view.app.read(cx).t(arrow::ERROR);
    notice("arrow.not-found", title, view.namespace.clone(), cx)
}
