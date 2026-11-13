//! Loading placeholder while the first home snapshot is being refreshed.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::home::HomeView;

pub fn skeleton(cx: &mut Context<HomeView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let block = |i: usize| {
        div()
            .id(("home-skeleton", i))
            .w(px(439.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(div().h(px(219.5)).rounded(px(12.)).bg(p.muted))
            .child(div().h(px(12.)).w(px(300.)).rounded(px(4.)).bg(p.muted))
    };
    div()
        .relative()
        .child(
            div()
                .h(px(22.))
                .w(px(110.))
                .mb(px(28.))
                .rounded(px(4.))
                .bg(p.muted),
        )
        .child(div().flex().gap(px(12.)).children((0..2).map(block)))
        .child(probe("home.skeleton", Kind::Other, "Loading", true, ""))
}
