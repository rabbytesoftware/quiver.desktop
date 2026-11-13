//! `PageHeader`: the title block at the top of a page (title, optional description, actions at the right).

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, FontWeight, SharedString, Window};

use crate::theme::ActiveQuiver;
use crate::tracked::TrackedText;

#[derive(IntoElement)]
pub struct PageHeader {
    title: SharedString,
    description: Option<SharedString>,
    actions: Vec<AnyElement>,
}

impl PageHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            actions: Vec::new(),
        }
    }

    pub fn description(
        mut self,
        text: impl Into<SharedString>,
    ) -> Self {
        self.description = Some(text.into());
        self
    }
}

impl ParentElement for PageHeader {
    /// Children are the actions at the right edge.
    fn extend(
        &mut self,
        elements: impl IntoIterator<Item = AnyElement>,
    ) {
        self.actions.extend(elements);
    }
}

impl RenderOnce for PageHeader {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        div()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(16.))
            .pb(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .min_w_0()
                    .child(
                        TrackedText::new(self.title, 24., 30., -0.4, p.foreground)
                            .weight(FontWeight::SEMIBOLD),
                    )
                    .when_some(self.description, |d, text| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(18.))
                                .text_color(p.muted_foreground)
                                .child(text),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .children(self.actions),
            )
    }
}
