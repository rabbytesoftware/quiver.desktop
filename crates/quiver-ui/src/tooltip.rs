//! `Tooltip`: port of `src/components/ui/tooltip.tsx`, on top of GPUI's hover tooltip.
//! GPUI positions the tooltip next to the pointer, not centred above the trigger (`side="top"`).

use gpui::prelude::*;
use gpui::{black, div, px, AnyView, App, Context, SharedString, Window};
use quiver_theme::Radius;

use crate::elevation::shadow_md;
use crate::theme::ActiveQuiver;

/// The popup: `rounded-md border bg-popover text-popover-foreground text-xs shadow-md/5 px-2 py-1`.
pub struct TooltipPopup {
    text: SharedString,
}

impl Render for TooltipPopup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        div()
            .rounded(px(Radius::MD))
            .border_1()
            .border_color(p.border)
            .bg(p.popover)
            .text_color(p.popover_foreground)
            .text_size(px(12.))
            .px(px(8.))
            .py(px(4.))
            .shadow(shadow_md(black().opacity(0.05)))
            .child(self.text.clone())
    }
}

/// Builds the tooltip view for `.tooltip(...)`:
/// `div().id("x").tooltip(quiver_ui::tooltip("Settings"))`.
pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| TooltipPopup { text: text.clone() }).into()
}
