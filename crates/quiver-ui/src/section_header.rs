//! `SectionHeader`: a shelf title (18px) on a hairline with an optional "View all" link at the right.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, CursorStyle, ElementId, FontWeight, SharedString, Window};

use crate::inspect::{probe, Kind};
use crate::theme::ActiveQuiver;
use crate::tracked::TrackedText;

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SectionHeader {
    id: ElementId,
    title: SharedString,
    link: Option<(SharedString, ClickFn, Option<SharedString>)>,
}

impl SectionHeader {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            link: None,
        }
    }

    /// The link on the right ("View all 13 ->"). `probe_id` is its e2e id.
    pub fn link(
        mut self,
        label: impl Into<SharedString>,
        probe_id: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.link = Some((label.into(), Rc::new(on_click), Some(probe_id.into())));
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let probe_id = format!("section.{}", self.id);
        div()
            .id(self.id.clone())
            .relative()
            .child(probe(probe_id, Kind::Other, self.title.clone(), true, ""))
            // The section's width, not its content's: a shelf with one tile must not pull the link next to the title.
            .w_full()
            .flex()
            .justify_between()
            .items_end()
            .pb(px(12.))
            .mb(px(16.))
            .border_b_1()
            .border_color(p.border)
            // The title takes the room the link leaves, so the link always ends the row.
            .child(
                div().flex_1().min_w_0().child(
                    TrackedText::new(self.title, 18., 22., -0.3, p.foreground)
                        .weight(FontWeight::SEMIBOLD),
                ),
            )
            .when_some(self.link, |d, (label, on_click, probe_id)| {
                d.child(
                    div()
                        .id((self.id, "link"))
                        .flex_none()
                        .relative()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.foreground)
                        .cursor(CursorStyle::PointingHand)
                        .hover(|s| s.text_color(p.muted_foreground))
                        .on_click(move |e, w, cx| on_click(e, w, cx))
                        .child(label.clone())
                        .when_some(probe_id, |d, id| {
                            d.child(probe(id, Kind::Button, label, true, ""))
                        }),
                )
            })
    }
}
