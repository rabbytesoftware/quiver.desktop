//! `Label`: port of `src/components/ui/label.tsx` (`text-sm leading-none font-medium`).

use gpui::prelude::*;
use gpui::{div, px, relative, App, FontWeight, SharedString, StyleRefinement, Window};

use crate::theme::ActiveQuiver;

#[derive(IntoElement)]
pub struct Label {
    text: SharedString,
    disabled: bool,
    style: StyleRefinement,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            disabled: false,
            style: StyleRefinement::default(),
        }
    }

    /// `peer-disabled:opacity-50`.
    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let mut el = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(14.))
            .line_height(relative(1.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(cx.quiver().palette.foreground)
            .when(self.disabled, |d| d.opacity(0.5))
            .child(self.text);
        crate::style_merge::apply(el.style(), &self.style);
        el
    }
}
