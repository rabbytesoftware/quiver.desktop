//! Focus handling and focus rings.
//!
//! CSS rings only on `:focus-visible` (keyboard). GPUI focuses on mouse down too, so interactive
//! controls that are not text fields call [`FocusableExt::no_pointer_focus`]: clicking them does
//! not move focus, Tab does, and the `focus` style (the ring) therefore only shows for the keyboard.

use gpui::{point, px, App, BoxShadow, ElementId, FocusHandle, Hsla, InteractiveElement, Window};

use crate::theme::ActiveQuiver;

/// A tab-stop focus handle that lives as long as the element with `id` keeps rendering.
/// (GPUI reads the tab-stop flag from the handle, not from the element, when a handle is tracked.)
pub fn keyed_focus(
    id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> FocusHandle {
    let key = ElementId::from((id.clone(), "focus"));
    window
        .use_keyed_state(key, cx, |_, cx| cx.focus_handle().tab_stop(true))
        .read(cx)
        .clone()
}

/// A focus handle for a container (popup, field wrapper): focusable by code and by click, but
/// skipped by Tab.
pub fn keyed_container_focus(
    id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> FocusHandle {
    let key = ElementId::from((id.clone(), "focus"));
    window
        .use_keyed_state(key, cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone()
}

pub trait FocusableExt: InteractiveElement + Sized {
    /// Mouse down does not focus this element (CSS buttons keep `:focus-visible` for the keyboard).
    fn no_pointer_focus(self) -> Self {
        self.capture_any_mouse_down(|_, window, _| window.prevent_default())
    }
}

impl<T: InteractiveElement + Sized> FocusableExt for T {}

fn spread(
    color: Hsla,
    px_: f32,
) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(px_),
    }
}

/// `focus-visible:ring-2 ring-ring ring-offset-1 ring-offset-background` (buttons, badges, tabs).
pub fn offset_ring(cx: &App) -> Vec<BoxShadow> {
    let p = cx.quiver().palette;
    vec![spread(p.ring, 3.0), spread(p.background, 1.0)]
}

/// A plain `ring-N` of `color` (inputs, switch, number field: `ring-3 ring-ring/50`).
pub fn ring(
    color: Hsla,
    width: f32,
) -> BoxShadow {
    spread(color, width)
}
