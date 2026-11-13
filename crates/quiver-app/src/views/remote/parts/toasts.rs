//! The transient confirmations at the bottom-right of the window (`fixed right-5 bottom-5 z-60`): mounted
//! by the shell, so a switch from the command palette toasts on whatever screen is showing, and above the
//! modal layer, so an Add that failed toasts over its still-open dialog.

use gpui::prelude::*;
use gpui::{anchored, div, point, px, App, Corner, IntoElement, Window};
use quiver_state::remote::ToastStack;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::layer;
use quiver_ui::{icon, ActiveQuiver, Glyph};

pub fn toasts(
    stack: &ToastStack,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let size = window.viewport_size();
    let column =
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(8.))
            .children(stack.toasts().iter().map(|toast| {
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.border)
                    .bg(p.popover)
                    .text_color(p.popover_foreground)
                    .text_size(px(12.5))
                    .shadow(quiver_ui::shadow_lg(gpui::black().opacity(0.05)))
                    .child(icon(Glyph::Check, 14., p.muted_foreground))
                    .child(toast.message.clone())
                    .child(probe(
                        format!("remote.toast.{}", toast.id),
                        Kind::Text,
                        toast.message.clone(),
                        true,
                        "",
                    ))
            }));
    // Above the dialogs' layer (priority 10), as z-60 is above z-50.
    layer(
        anchored()
            .anchor(Corner::BottomRight)
            .position(point(size.width - px(20.), size.height - px(20.)))
            .child(column),
    )
    .with_priority(20)
}
