//! What Menu and Select share: open state, keyboard navigation, the popup surface and its placement.

use gpui::prelude::*;
use gpui::{
    anchored, black, deferred, div, px, AnyElement, App, Corner, ElementId, FocusHandle, Window,
};
use quiver_theme::Radius;

use crate::elevation::shadow_lg;
use crate::focus::{keyed_container_focus, keyed_focus};
use crate::ids::child_id;
use crate::theme::ActiveQuiver;

#[derive(Debug, Default)]
pub(crate) struct PopupState {
    pub open: bool,
    pub highlighted: Option<usize>,
    /// Whether the popup was open when the current mouse press began (outside presses close it
    /// before the trigger's click handler runs, which must not reopen it).
    pub was_open_at_press: bool,
    /// Enter/Space chose an item on key down; the key-up that follows lands on the refocused
    /// trigger as a keyboard click, which must not reopen the popup.
    pub swallow_click: bool,
}

impl PopupState {
    pub fn open_at(&mut self, highlighted: Option<usize>, cx: &mut gpui::Context<Self>) {
        self.open = true;
        self.highlighted = highlighted;
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut gpui::Context<Self>) {
        self.open = false;
        self.highlighted = None;
        cx.notify();
    }
}

pub(crate) type OpenFn = std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Opens or closes the popup and reports a change through `on_open_change` (`onOpenChange`).
pub(crate) fn set_open(
    state: &gpui::Entity<PopupState>,
    open: bool,
    highlighted: Option<usize>,
    on_open_change: &Option<OpenFn>,
    window: &mut Window,
    cx: &mut App,
) {
    let changed = state.update(cx, |s, cx| {
        let changed = s.open != open;
        if open {
            s.open_at(highlighted, cx)
        } else {
            s.close(cx)
        }
        changed
    });
    if let (true, Some(f)) = (changed, on_open_change) {
        f(open, window, cx);
    }
}

pub(crate) struct PopupHandles {
    pub state: gpui::Entity<PopupState>,
    pub popup_focus: FocusHandle,
    pub trigger_focus: FocusHandle,
}

pub(crate) fn use_popup(
    id: &ElementId,
    trigger_id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> PopupHandles {
    PopupHandles {
        state: window.use_keyed_state(child_id(id, "popup-state"), cx, |_, _| {
            PopupState::default()
        }),
        popup_focus: keyed_container_focus(&child_id(id, "popup"), window, cx),
        trigger_focus: keyed_focus(trigger_id, window, cx),
    }
}

/// What a key press means inside an open list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nav {
    Highlight(usize),
    Activate,
    Close,
    Ignore,
}

fn step(from: Option<usize>, dir: isize, disabled: &[bool]) -> Option<usize> {
    let n = disabled.len() as isize;
    if n == 0 {
        return None;
    }
    let start = from.map_or(if dir > 0 { -1 } else { n }, |i| i as isize);
    (1..=n)
        .map(|k| start + dir * k)
        .filter(|i| (0..n).contains(i))
        .map(|i| i as usize)
        .find(|i| !disabled[*i])
}

pub(crate) fn nav_key(key: &str, highlighted: Option<usize>, disabled: &[bool]) -> Nav {
    let pick = |i: Option<usize>| i.map_or(Nav::Ignore, Nav::Highlight);
    match key {
        "down" => pick(step(highlighted, 1, disabled)),
        "up" => pick(step(highlighted, -1, disabled)),
        "home" => pick(step(None, 1, disabled)),
        "end" => pick(step(None, -1, disabled)),
        "enter" | "space" => highlighted.map_or(Nav::Ignore, |_| Nav::Activate),
        "escape" => Nav::Close,
        _ => Nav::Ignore,
    }
}

/// `rounded-lg border bg-popover p-1 text-popover-foreground shadow-lg/5`.
pub(crate) fn popup_surface(cx: &App) -> gpui::Div {
    let p = cx.quiver().palette;
    div()
        .rounded(px(Radius::LG))
        .border_1()
        .border_color(p.border)
        .bg(p.popover)
        .text_color(p.popover_foreground)
        .p(px(4.))
        .shadow(shadow_lg(black().opacity(0.05)))
}

/// Places `popup` 4px under `trigger`, aligned to its start or end edge, above everything else.
pub(crate) fn floating(
    trigger: AnyElement,
    popup: Option<AnyElement>,
    align_end: bool,
) -> gpui::Div {
    let corner = if align_end {
        Corner::TopRight
    } else {
        Corner::TopLeft
    };
    div().relative().child(trigger).children(popup.map(|popup| {
        let holder = div()
            .absolute()
            .top_full()
            .mt(px(4.))
            .when(align_end, |d| d.right_0())
            .when(!align_end, |d| d.left_0())
            .min_w_full();
        holder.child(
            deferred(
                anchored()
                    .anchor(corner)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup),
            )
            .with_priority(1),
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_skips_disabled_and_stops_at_the_end() {
        let d = [false, true, false];
        assert_eq!(nav_key("down", None, &d), Nav::Highlight(0));
        assert_eq!(nav_key("down", Some(0), &d), Nav::Highlight(2));
        assert_eq!(nav_key("down", Some(2), &d), Nav::Ignore);
        assert_eq!(nav_key("up", Some(2), &d), Nav::Highlight(0));
    }

    #[test]
    fn home_end_enter_escape() {
        let d = [true, false, false];
        assert_eq!(nav_key("home", None, &d), Nav::Highlight(1));
        assert_eq!(nav_key("end", None, &d), Nav::Highlight(2));
        assert_eq!(nav_key("enter", Some(1), &d), Nav::Activate);
        assert_eq!(nav_key("enter", None, &d), Nav::Ignore);
        assert_eq!(nav_key("escape", None, &d), Nav::Close);
    }
}
