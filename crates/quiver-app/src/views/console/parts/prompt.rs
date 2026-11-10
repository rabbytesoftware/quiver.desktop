//! The prompt line: `>` and the input. Enter runs, Up/Down walk the history, Tab completes, Ctrl+C
//! cancels (see `keys`).

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, Window};
use quiver_state::console::Support;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Input};

use crate::views::console::ConsoleDock;

pub fn prompt(
    dock: &mut ConsoleDock,
    _: &mut Window,
    cx: &mut Context<ConsoleDock>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let enabled = dock.controller.state.support != Support::Unsupported;
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(36.))
        .px(px(12.))
        .border_t_1()
        .border_color(p.border)
        .font_family(FontFamily::Mono.family())
        .text_size(px(12.5))
        .child(div().text_color(p.primary).child(">"))
        .child(
            div()
                .relative()
                .flex_1()
                .child(Input::new("console-input", &dock.input))
                .child(probe("console.input", Kind::Input, "command", enabled, "")),
        )
        .child(div().relative().child(div().size_0()).child(probe(
            "console.run",
            Kind::Button,
            "run",
            enabled,
            "",
        )))
}
