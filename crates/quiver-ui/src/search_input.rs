//! `SearchInput`: the Crowbar search field. 34px, surface background, 1px border, 8px radius, a muted
//! magnifier, and a clear button once there is text. Text editing comes from [`InputState`].

use gpui::prelude::*;
use gpui::Focusable as _;
use gpui::{black, div, px, App, CursorStyle, ElementId, Entity, Window};
use gpui_component::input::Input as Editor;

use crate::elevation::shadow_xs;
use crate::icon::{icon, Glyph};
use crate::input::InputState;
use crate::inspect::{probe, Kind};
use crate::theme::ActiveQuiver;

#[derive(IntoElement)]
pub struct SearchInput {
    id: ElementId,
    state: Entity<InputState>,
    probe_id: Option<gpui::SharedString>,
    clear_probe_id: Option<gpui::SharedString>,
}

impl SearchInput {
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            probe_id: None,
            clear_probe_id: None,
        }
    }

    /// E2e ids of the field and of its clear button.
    pub fn probes(
        mut self,
        field: impl Into<gpui::SharedString>,
        clear: impl Into<gpui::SharedString>,
    ) -> Self {
        self.probe_id = Some(field.into());
        self.clear_probe_id = Some(clear.into());
        self
    }
}

impl RenderOnce for SearchInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.quiver().palette;
        let value = self.state.read(cx).value().to_string();
        let has_text = !value.is_empty();
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let state = self.state.clone();
        let clear = has_text.then(|| {
            div()
                .id((self.id.clone(), "clear"))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.))
                .rounded(px(6.))
                .text_color(p.muted_foreground)
                .cursor(CursorStyle::PointingHand)
                .hover(|s| s.bg(p.accent))
                .on_click(move |_, window, cx| {
                    state.update(cx, |s, cx| s.set_value("", window, cx));
                })
                .child(icon(Glyph::Close, 13., p.muted_foreground))
                .when_some(self.clear_probe_id.clone(), |d, id| {
                    d.child(probe(id, Kind::Button, "Clear search", true, ""))
                })
        });
        div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .gap(px(7.))
            .h(px(34.))
            .w_full()
            .pl(px(10.))
            .pr(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(if focused || has_text {
                p.foreground
            } else {
                p.input
            })
            .bg(p.background)
            .text_color(p.foreground)
            .shadow(shadow_xs(black().opacity(0.05)))
            .child(icon(Glyph::Search, 15., p.muted_foreground))
            .child(
                div().flex_1().min_w_0().child(
                    Editor::new(&self.state)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .text_size(px(13.)),
                ),
            )
            .children(clear)
            .when_some(self.probe_id, |d, id| {
                // The state is the typed value, then " [focused]" while the field holds the keyboard focus.
                let state = if focused {
                    format!("{value} [focused]")
                } else {
                    value
                };
                d.child(probe(id, Kind::Input, "Search", true, state))
            })
    }
}
