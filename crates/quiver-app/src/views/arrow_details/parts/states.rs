//! What the page shows before it has an arrow: the loading skeleton, a failed read, and 404.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::i18n::en::{app, arrow, nav};
use quiver_state::queries::keys::arrow_detail_prefix;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant};

use crate::model::Route;
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
    body: String,
    action: Button,
    cx: &Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .py(px(120.))
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .max_w(px(420.))
                .text_size(px(13.))
                .text_color(p.muted_foreground)
                .child(body),
        )
        .child(div().mt(px(8.)).child(action))
        .child(probe(id, Kind::Other, id, true, ""))
}

/// The daemon could not be read, or answered with an error.
pub fn load_error(
    view: &mut ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let model = view.app.read(cx);
    let (title, retry) = (model.t(arrow::ERROR), model.t(app::CRASH_RETRY));
    let body = view.load.read(|l| l.error.clone()).unwrap_or_default();
    let queries = model.services.queries.clone();
    let action = Button::new("arrow.error.retry")
        .variant(ButtonVariant::Outline)
        .label(retry.clone())
        .child(probe("arrow.error.retry", Kind::Button, retry, true, ""))
        .on_click(move |_, _, _| queries.invalidate(&arrow_detail_prefix()));
    notice("arrow.error", title, body, action, cx)
}

/// The daemon (or the library) has no arrow at this namespace.
pub fn not_found(
    view: &mut ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let model = view.app.read(cx);
    let (title, home) = (model.t(arrow::ERROR), model.t(nav::HOME));
    let app = view.app.clone();
    let action = Button::new("arrow.not-found.home")
        .variant(ButtonVariant::Outline)
        .label(home.clone())
        .child(probe("arrow.not-found.home", Kind::Button, home, true, ""))
        .on_click(move |_, _, cx| app.update(cx, |m, cx| m.navigate(Route::Home, cx)));
    notice("arrow.not-found", title, view.namespace.clone(), action, cx)
}
