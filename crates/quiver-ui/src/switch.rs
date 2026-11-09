//! `Switch`: port of `src/components/ui/switch.tsx` (2 sizes). Space toggles; Enter does not.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, App, ClickEvent, CursorStyle, ElementId, KeyboardButton, StyleRefinement, Window,
};

use crate::focus::{keyed_focus, ring, FocusableExt};
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchSize {
    #[default]
    Default,
    Sm,
}

impl SwitchSize {
    /// (track width, track height, thumb size) in px.
    fn metrics(self) -> (f32, f32, f32) {
        match self {
            SwitchSize::Default => (32.0, 18.4, 16.0),
            SwitchSize::Sm => (24.0, 14.0, 12.0),
        }
    }
}

type ChangeFn = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    size: SwitchSize,
    disabled: bool,
    on_change: Option<ChangeFn>,
    style: StyleRefinement,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            size: SwitchSize::Default,
            disabled: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the next value when the user toggles the switch.
    pub fn on_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let q = *cx.quiver();
        let p = q.palette;
        let (w, h, thumb) = self.size.metrics();
        let focus = keyed_focus(&self.id, window, cx);
        let (bg, thumb_bg) = match (self.checked, q.is_dark()) {
            (true, true) => (p.primary, p.primary_foreground),
            (true, false) => (p.primary, p.background),
            (false, true) => (p.input.opacity(0.8), p.foreground),
            (false, false) => (p.input, p.background),
        };
        let travel = thumb - 2.0;
        let key = self.id.to_string();
        let next = !self.checked;
        let on_change = self.on_change.clone();
        let focus_ring = ring(p.ring.opacity(0.5), 3.0);
        let mut el = div()
            .id(self.id)
            .debug_selector(move || key)
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .w(px(w))
            .h(px(h))
            .rounded_full()
            .border_1()
            .border_color(gpui::transparent_black())
            .bg(bg)
            .shadow(crate::elevation::shadow_xs(gpui::black().opacity(0.05)))
            .child(
                div()
                    .size(px(thumb))
                    .flex_none()
                    .rounded_full()
                    .bg(thumb_bg)
                    .ml(px(if self.checked { travel } else { 0.0 })),
            );
        el = if self.disabled {
            el.opacity(0.5).cursor(CursorStyle::OperationNotAllowed)
        } else {
            el.cursor_pointer()
                .track_focus(&focus)
                .tab_stop(true)
                .no_pointer_focus()
                .focus(move |mut s| {
                    s.border_color = Some(p.ring);
                    s.box_shadow = Some(vec![focus_ring]);
                    s
                })
                .on_click(move |e: &ClickEvent, window, cx| {
                    if matches!(e, ClickEvent::Keyboard(k) if k.button == KeyboardButton::Enter) {
                        return;
                    }
                    if let Some(f) = &on_change {
                        f(next, window, cx);
                    }
                })
        };
        el.style().refine(&self.style);
        el
    }
}
