//! Home when there is nothing to show: empty library, or Core could not be reached.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::i18n::en::{app, home as keys, sidebar};
use quiver_state::queries::keys::home_key;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant};

use crate::model::Route;
use crate::views::home::HomeView;

fn notice(
    title: String,
    body: String,
    action: Button,
    screen: &'static str,
    cx: &Context<HomeView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
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
        .child(div().mt(px(8.)).child(action))
        .child(probe(screen, Kind::Other, screen, true, ""))
}

pub fn empty_state(
    view: &mut HomeView,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let a = view.app.read(cx);
    let (title, body, cta) = (
        a.t(keys::EMPTY_TITLE),
        a.t(keys::EMPTY_DESCRIPTION),
        a.t(keys::EMPTY_CTA),
    );
    let go = view.app.clone();
    let action = Button::new("home.empty.search")
        .label(cta.clone())
        .child(probe("home.empty.search", Kind::Button, cta, true, ""))
        .on_click(move |_, _, cx| {
            go.update(cx, |m, cx| {
                m.navigate(
                    Route::Search {
                        query: String::new(),
                    },
                    cx,
                )
            });
        });
    notice(title, body, action, "home.empty", cx)
}

pub fn error_state(
    view: &mut HomeView,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let a = view.app.read(cx);
    let (title, body, retry) = (
        a.t(sidebar::ARROWS_ERROR_TITLE),
        a.t(sidebar::ARROWS_ERROR_BODY),
        a.t(app::CRASH_RETRY),
    );
    let queries = a.services.queries.clone();
    let action = Button::new("home.error.retry")
        .variant(ButtonVariant::Outline)
        .label(retry.clone())
        .child(probe("home.error.retry", Kind::Button, retry, true, ""))
        .on_click(move |_, _, _| queries.invalidate(&home_key()));
    notice(title, body, action, "home.error", cx)
}
