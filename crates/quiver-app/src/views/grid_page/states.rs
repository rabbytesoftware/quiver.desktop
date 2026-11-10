use gpui::prelude::*;
use gpui::{div, px, App, Entity, FontWeight, IntoElement, SharedString};
use quiver_state::i18n::en::{app, sidebar};
use quiver_state::queries::keys::QueryKey;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant};

use crate::model::AppModel;

/// Title, body and an optional action, centred; `id` is its e2e probe (`<screen>.empty`, `.error`).
pub fn notice(
    id: impl Into<SharedString>,
    title: String,
    body: String,
    action: Option<Button>,
    cx: &App,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let id: SharedString = id.into();
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .py(px(96.))
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(p.muted_foreground)
                .child(body),
        )
        .children(action.map(|a| div().mt(px(8.)).child(a)))
        .child(probe(id.clone(), Kind::Other, id, true, ""))
}

/// The error state of a Core read: the same words as Home, a retry that invalidates `key`.
pub fn error_state(
    screen: &str,
    app_model: &Entity<AppModel>,
    key: QueryKey,
    cx: &App,
) -> impl IntoElement {
    let a = app_model.read(cx);
    let (title, body, retry) = (
        a.t(sidebar::ARROWS_ERROR_TITLE),
        a.t(sidebar::ARROWS_ERROR_BODY),
        a.t(app::CRASH_RETRY),
    );
    let queries = a.services.queries.clone();
    let id = format!("{screen}.error.retry");
    let action = Button::new(SharedString::from(id.clone()))
        .variant(ButtonVariant::Outline)
        .label(retry.clone())
        .child(probe(id, Kind::Button, retry, true, ""))
        .on_click(move |_, _, _| queries.invalidate(&key));
    notice(format!("{screen}.error"), title, body, Some(action), cx)
}

/// `count` placeholder tiles of `width` while the first read is in flight.
pub fn skeleton(screen: &str, count: usize, width: f32, cx: &App) -> impl IntoElement {
    let p = cx.quiver().palette;
    let block = move |i: usize| {
        div()
            .id(("skeleton", i))
            .w(px(width))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(div().h(px(width / 2.)).rounded(px(12.)).bg(p.muted))
            .child(
                div()
                    .h(px(12.))
                    .w(px(width * 0.7))
                    .rounded(px(4.))
                    .bg(p.muted),
            )
    };
    let id = format!("{screen}.skeleton");
    super::tile_grid((0..count).map(|i| block(i).into_any_element()))
        .relative()
        .child(probe(id, Kind::Other, "Loading", true, ""))
}
