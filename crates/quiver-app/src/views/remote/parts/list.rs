//! The page header, the connection list and its loading / empty states.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::i18n::en::{remote, screens};
use quiver_state::remote::ConnectionRowView;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Button, ButtonVariant, Glyph, TrackedText};

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
        .mb(px(4.))
        .pb(px(14.))
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    // Board RemoteAfter: letter-spacing -.4px.
                    TrackedText::new(title, 24., 28., -0.4, p.foreground)
                        .weight(FontWeight::SEMIBOLD),
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
                .child(icon(Glyph::Plus, 16., p.primary_foreground))
                .label(add.clone())
                .on_click(move |_, window, cx| open_add(this.clone(), window, cx))
                .child(probe("remote.add", Kind::Button, add, true, "")),
        )
}

pub fn loading(cx: &Context<RemoteView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let line = move |width: f32| div().h(px(10.)).w(px(width)).rounded(px(2.)).bg(p.muted);
    div()
        .flex()
        .flex_col()
        .children(
            [96., 132., 112.]
                .into_iter()
                .enumerate()
                .map(move |(i, w)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .py(px(12.))
                        .when(i < 2, |d| d.border_b_1().border_color(p.border))
                        .child(div().flex_none().size(px(28.)).rounded(px(7.)).bg(p.muted))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(line(w))
                                .child(line(w * 1.3)),
                        )
                }),
        )
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
        .gap(px(4.))
        .px(px(20.))
        .pt(px(36.))
        .pb(px(8.))
        .child(
            div()
                .mb(px(10.))
                .size(px(44.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.border)
                .bg(p.muted)
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Glyph::Globe, 20., p.muted_foreground)),
        )
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(
            div()
                .mb(px(16.))
                .max_w(px(320.))
                .text_center()
                .text_size(px(12.))
                .line_height(px(19.5))
                .text_color(p.muted_foreground)
                .child(body),
        )
        .child(
            Button::new("remote-empty-add")
                .variant(ButtonVariant::Outline)
                .label(add.clone())
                .on_click(move |_, window, cx| open_add(this.clone(), window, cx))
                .child(probe("remote.empty.add", Kind::Button, add, true, "")),
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
    let hint = (!rows.iter().any(|row| row.is_remote)).then(|| empty(view, cx));
    div().flex().flex_col().children(items).children(hint)
}
