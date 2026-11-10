//! The transient confirmations at the bottom of the screen.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::remote::RemoteView;

pub fn toasts(view: &RemoteView, cx: &Context<RemoteView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .absolute()
        .bottom(px(24.))
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .children(view.toasts.toasts().iter().map(|toast| {
            div()
                .relative()
                .px(px(14.))
                .py(px(9.))
                .rounded(px(9.))
                .bg(p.primary)
                .text_color(p.primary_foreground)
                .text_size(px(13.))
                .shadow_md()
                .child(toast.message.clone())
                .child(probe(
                    format!("remote.toast.{}", toast.id),
                    Kind::Text,
                    toast.message.clone(),
                    true,
                    "",
                ))
        }))
}
