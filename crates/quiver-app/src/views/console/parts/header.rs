//! The dock's title bar: its name, then Cancel (while a command runs), Clear and Close.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::i18n::en::{console, screens};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Button, ButtonSize, ButtonVariant, Glyph};

use crate::views::console::ConsoleDock;

type DockAction = Box<dyn Fn(&mut ConsoleDock, &mut Context<ConsoleDock>)>;

pub fn header(
    dock: &mut ConsoleDock,
    cx: &mut Context<ConsoleDock>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let m = dock.app.read(cx);
    let (title, cancel, clear, running) = (
        m.t(console::PANEL_LABEL),
        m.t(screens::CONSOLE_CANCEL),
        m.t(screens::CONSOLE_CLEAR),
        m.t(screens::CONSOLE_RUNNING),
    );
    let busy = dock.controller.state.running > 0;
    let this = cx.entity();
    let button =
        |id: &'static str, probe_id: &'static str, label: String, enabled: bool, f: DockAction| {
            let this = this.clone();
            div()
                .relative()
                .child(
                    Button::new(id)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm)
                        .disabled(!enabled)
                        .label(label.clone())
                        .on_click(move |_, _, cx| this.update(cx, |d, cx| f(d, cx))),
                )
                .child(probe(probe_id, Kind::Button, label, enabled, ""))
        };
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(34.))
        .px(px(12.))
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .when(busy, |d| {
            d.child(
                div()
                    .text_size(px(11.5))
                    .text_color(p.muted_foreground)
                    .child(format!("{running} ({})", dock.controller.state.running)),
            )
        })
        .child(div().flex_1())
        .child(button(
            "console-cancel",
            "console.cancel",
            cancel,
            busy,
            Box::new(|d, cx| d.cancel(cx)),
        ))
        .child(button(
            "console-clear",
            "console.clear",
            clear,
            true,
            Box::new(|d, cx| {
                d.controller.state.clear();
                cx.notify();
            }),
        ))
        .child(
            div()
                .relative()
                .child(
                    Button::new("console-close")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .child(icon(Glyph::Close, 14., p.muted_foreground))
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| this.update(cx, |d, cx| d.close(cx))
                        }),
                )
                .child(probe("console.close", Kind::Button, "close", true, "")),
        )
}
