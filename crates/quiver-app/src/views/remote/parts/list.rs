//! The page header, the connection list and its loading / empty states.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::i18n::en::{remote, screens};
use quiver_state::remote::ConnectionRowView;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant, Frame, FramePanel, Spinner};

use super::add_modal::open_add;
use super::row::connection_row;
use crate::views::remote::RemoteView;

pub fn header(
    view: &RemoteView,
    cx: &mut Context<RemoteView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let (title, subtitle, add) = {
        let m = view.app.read(cx);
        (
            m.t(remote::TITLE),
            m.t(screens::REMOTE_SUBTITLE),
            m.t(remote::ADD_BUTTON),
        )
    };
    let this = cx.entity();
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.))
        .mb(px(16.))
        .pb(px(10.))
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(20.))
                        .line_height(px(28.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .text_size(px(12.5))
                        .text_color(p.muted_foreground)
                        .child(subtitle),
                ),
        )
        .child(
            Button::new("remote-add")
                .variant(ButtonVariant::Default)
                .label(format!("+ {add}"))
                .on_click(move |_, window, cx| open_add(this.clone(), window, cx))
                .child(probe("remote.add", Kind::Button, add, true, "")),
        )
}

pub fn loading(cx: &Context<RemoteView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .flex()
        .justify_center()
        .py(px(64.))
        .text_color(p.muted_foreground)
        .child(Spinner::new("remote-loading").size(20.))
}

pub fn empty(
    view: &RemoteView,
    cx: &mut Context<RemoteView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let (title, body, add) = {
        let m = view.app.read(cx);
        (
            m.t(remote::EMPTY_TITLE),
            m.t(remote::EMPTY_DESCRIPTION),
            m.t(remote::ADD_BUTTON),
        )
    };
    let this = cx.entity();
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .py(px(72.))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(p.muted_foreground)
                .child(body),
        )
        .child(
            div().mt(px(8.)).child(
                Button::new("remote-empty-add")
                    .variant(ButtonVariant::Outline)
                    .label(add.clone())
                    .on_click(move |_, window, cx| open_add(this.clone(), window, cx))
                    .child(probe("remote.empty.add", Kind::Button, add, true, "")),
            ),
        )
        .child(probe("remote.empty", Kind::Other, "empty", true, ""))
}

pub fn rows(
    view: &RemoteView,
    rows: &[ConnectionRowView],
    cx: &mut Context<RemoteView>,
) -> impl IntoElement {
    let last = rows.len().saturating_sub(1);
    let items: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| connection_row(view, row, i == last, cx))
        .collect();
    Frame::new().child(
        FramePanel::new()
            .py(px(8.))
            .flex()
            .flex_col()
            .children(items),
    )
}
