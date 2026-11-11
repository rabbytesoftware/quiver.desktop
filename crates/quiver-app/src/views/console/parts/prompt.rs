//! The prompt line: `>` and the input. Enter runs, Up/Down walk the history, Tab completes, Ctrl+C
//! cancels (see `keys`).

use gpui::prelude::*;
use gpui::{div, px, Context, Focusable as _, IntoElement, Window};
use quiver_state::console::entries::StreamState;
use quiver_state::console::Support;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::console::ConsoleDock;

pub fn prompt(
    dock: &mut ConsoleDock,
    window: &mut Window,
    cx: &mut Context<ConsoleDock>,
) -> impl IntoElement {
    // `[focused]` in the probe's state lets the e2e driver wait for a click to have given the field the keyboard.
    let focused = dock.input.read(cx).focus_handle(cx).is_focused(window);
    let p = cx.quiver().palette;
    let colors = super::Colors::of(&quiver_theme::Tokens::for_mode(cx.quiver().mode));
    let enabled = dock.controller.state.support != Support::Unsupported;
    let reconnecting = dock.controller.state.stream == StreamState::Reconnecting;
    let reconnecting_label = dock
        .app
        .read(cx)
        .t(quiver_state::i18n::en::console::STREAM_RECONNECTING);
    let hover = colors.hover;
    let dim = colors.dim;
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(10.))
        .h(px(35.))
        .px(px(16.))
        .border_t_1()
        .border_color(p.border)
        .when(focused, |d| d.bg(hover))
        .font_family(FontFamily::Mono.family())
        .text_size(px(13.))
        .text_color(colors.foreground)
        .child(div().text_size(px(14.)).child("›"))
        .child(
            div()
                .relative()
                .flex_1()
                .child(
                    gpui_component::input::Input::new(&dock.input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .h(px(34.)),
                )
                .child(probe(
                    "console.input",
                    Kind::Input,
                    "command",
                    enabled,
                    if focused { "[focused]" } else { "" },
                )),
        )
        .when(reconnecting, |d| {
            d.child(
                div()
                    .text_size(px(11.))
                    .text_color(dim)
                    .child(reconnecting_label),
            )
        })
        .child(div().relative().child(div().size_0()).child(probe(
            "console.run",
            Kind::Button,
            "run",
            enabled,
            "",
        )))
}
