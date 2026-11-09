//! The modal layer shared by Dialog and Sheet: a window-sized backdrop above everything else.
//!
//! GPUI has no backdrop blur, so `bg-black/32 backdrop-blur-sm` is the dim only.

use gpui::prelude::*;
use gpui::{
    anchored, black, deferred, div, point, px, relative, Animation, AnimationExt, AnyElement, App,
    ElementId, FocusHandle, KeyDownEvent, MouseButton, Window,
};
use std::rc::Rc;
use std::time::Duration;

use crate::focus::keyed_container_focus;
use crate::ids::child_id;

pub(crate) type DismissFn = Rc<dyn Fn(&mut Window, &mut App)>;

pub(crate) const ENTER: Duration = Duration::from_millis(200);

/// How the popup sits inside the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Centred horizontally, 1fr above / 3fr below.
    Dialog,
    Right,
    Left,
    Top,
    Bottom,
}

pub(crate) struct ModalLayer {
    pub id: ElementId,
    pub placement: Placement,
    /// Padding around the popup (`p-4` for dialogs, `sm:p-4` for inset sheets).
    pub inset: f32,
    pub popup: AnyElement,
    pub on_dismiss: Option<DismissFn>,
}

/// Focus handle of the popup, which receives Escape.
pub(crate) fn popup_focus(id: &ElementId, window: &mut Window, cx: &mut App) -> FocusHandle {
    keyed_container_focus(&child_id(id, "modal"), window, cx)
}

/// Escape dismisses; every other key stays inside the modal.
pub(crate) fn escape_handler(
    on_dismiss: Option<DismissFn>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) {
    move |e, window, cx| {
        if e.keystroke.key == "escape" {
            if let Some(f) = &on_dismiss {
                f(window, cx);
            }
        }
    }
}

/// A flex spacer taking `grow` shares of the free space (`grid-rows-[1fr_auto_3fr]`).
fn spacer(grow: f32) -> gpui::Div {
    let mut d = div().flex_basis(relative(0.)).w_full().min_h_0();
    d.style().flex_grow = Some(grow);
    d
}

impl ModalLayer {
    pub fn into_element(self, window: &mut Window, cx: &mut App) -> AnyElement {
        let focus = popup_focus(&self.id, window, cx);
        if !focus.contains_focused(window, cx) {
            let f = focus.clone();
            window.defer(cx, move |window, _| window.focus(&f));
        }
        let size = window.viewport_size();
        let dismiss = self.on_dismiss.clone();
        let (vw, vh) = (size.width, size.height);
        let base = div()
            .flex()
            .size_full()
            .p(px(self.inset))
            .bg(black().opacity(0.32));
        let base = match self.placement {
            Placement::Dialog => base
                .flex_col()
                .items_center()
                .child(spacer(1.0))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .min_h_0()
                        .child(self.popup),
                )
                .child(spacer(3.0)),
            Placement::Right => base.justify_end().child(self.popup),
            Placement::Left => base.justify_start().child(self.popup),
            Placement::Top => base.flex_col().justify_start().child(self.popup),
            Placement::Bottom => base.flex_col().justify_end().child(self.popup),
        };
        let layer = base
            .id(child_id(&self.id, "backdrop"))
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if let Some(f) = &dismiss {
                    f(window, cx);
                }
            })
            .with_animation(
                child_id(&self.id, "fade"),
                Animation::new(ENTER),
                |el, t| el.opacity(t),
            );
        deferred(
            anchored()
                .position(point(px(0.), px(0.)))
                .child(div().w(vw).h(vh).child(layer)),
        )
        .with_priority(10)
        .into_any_element()
    }
}
