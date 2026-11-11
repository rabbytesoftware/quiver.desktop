//! `Input`: port of `src/components/ui/input.tsx` (h-9 text field with ring on focus).
//!
//! Text editing, selection, clipboard and IME come from gpui-component's `InputState`; this file
//! only supplies the Coss box around it (border, background, shadow, focus and invalid rings).

use gpui::prelude::*;
use gpui::{black, div, px, App, ElementId, Entity, StyleRefinement, Window};
use gpui_component::input::Input as Editor;
pub use gpui_component::input::{InputEvent, InputState};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::focus::{keyed_container_focus, ring};
use crate::theme::ActiveQuiver;

#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    invalid: bool,
    style: StyleRefinement,
}

impl Input {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<InputState>,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            disabled: false,
            invalid: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }

    /// `aria-invalid`: destructive border and ring.
    pub fn invalid(
        mut self,
        invalid: bool,
    ) -> Self {
        self.invalid = invalid;
        self
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let q = *cx.quiver();
        let p = q.palette;
        let within = keyed_container_focus(&self.id, window, cx);
        let (border, ring_color) = if self.invalid {
            (
                p.destructive.opacity(if q.is_dark() { 0.5 } else { 1.0 }),
                p.destructive.opacity(if q.is_dark() { 0.4 } else { 0.2 }),
            )
        } else {
            (p.input, p.ring.opacity(0.5))
        };
        let focus_border = if self.invalid { border } else { p.ring };
        let key = self.id.to_string();
        let mut el = div()
            .id(self.id)
            .debug_selector(move || key)
            .track_focus(&within)
            .flex()
            .items_center()
            .w_full()
            .min_w_0()
            .h(px(36.))
            .rounded(px(Radius::MD))
            .border_1()
            .border_color(border)
            .bg(if q.is_dark() {
                p.background.blend(p.input.opacity(0.3))
            } else {
                p.background
            })
            .text_size(px(14.))
            .text_color(p.foreground)
            .shadow(shadow_xs(black().opacity(0.05)))
            .child(crate::edge::control_edge(&q, Radius::MD, 0.06))
            .when(self.invalid, |d| {
                d.map(|mut d| {
                    d.style().box_shadow = Some(vec![ring(ring_color, 3.0)]);
                    d
                })
            })
            .in_focus(move |mut s| {
                s.border_color = Some(focus_border);
                s.box_shadow = Some(vec![ring(ring_color, 3.0)]);
                s
            })
            .when(self.disabled, |d| d.opacity(0.5))
            .child(
                Editor::new(&self.state)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false)
                    .disabled(self.disabled)
                    .px(px(10.))
                    .h_full(),
            );
        el.style().refine(&self.style);
        el
    }
}
