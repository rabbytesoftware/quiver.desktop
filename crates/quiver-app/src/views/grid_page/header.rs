use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, CursorStyle, Entity, IntoElement, SharedString};
use quiver_state::i18n::en::nav;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Glyph, PageHeader};

use crate::model::{AppModel, Route};

/// "<- Home" above the title. `screen` namespaces its id (`<screen>.back`).
pub fn back_link(
    app: &Entity<AppModel>,
    screen: &str,
    cx: &App,
) -> impl IntoElement {
    let muted = cx.quiver().palette.muted_foreground;
    let label = app.read(cx).t(nav::HOME);
    let go = app.clone();
    let id: SharedString = format!("{screen}.back").into();
    div()
        .id(SharedString::from(format!("{screen}-back")))
        .relative()
        .flex()
        .items_center()
        .gap(px(5.))
        .h(px(16.))
        .text_size(px(12.))
        .text_color(muted)
        .cursor(CursorStyle::PointingHand)
        .on_click(move |_, _, cx| {
            go.update(cx, |m, cx| m.navigate(Route::Home, cx));
        })
        .child(icon(Glyph::ArrowLeft, 13., muted))
        .child(label.clone())
        .child(probe(id, Kind::Button, label, true, ""))
}

/// The back link, then the title block.
pub fn page_header(
    app: &Entity<AppModel>,
    screen: &str,
    title: String,
    subtitle: Option<String>,
    actions: Vec<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let mut header = PageHeader::new(title);
    if let Some(text) = subtitle {
        header = header.description(text);
    }
    div()
        .flex()
        .flex_col()
        .gap(px(9.))
        .child(back_link(app, screen, cx))
        .child(header.children(actions))
}
