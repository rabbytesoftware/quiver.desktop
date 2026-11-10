use gpui::prelude::*;
use gpui::{div, Context, Modifiers, TestAppContext, Window};
use quiver_ui::{TabItem, Tabs};

use crate::{centre, focus_first, open, tap};

struct Host {
    selected: usize,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().child(
            Tabs::new("tabs")
                .selected(self.selected)
                .tab(TabItem::new("General"))
                .tab(TabItem::new("Engine").disabled(true))
                .tab(TabItem::new("Developer"))
                .on_change(cx.processor(|this, i, _, cx| {
                    this.selected = i;
                    cx.notify();
                })),
        )
    }
}

#[gpui::test]
fn click_selects(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { selected: 0 });
    let at = centre(cx, "tabs-2");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.selected), 2);
}

#[gpui::test]
fn disabled_tab_cannot_be_selected(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { selected: 0 });
    let at = centre(cx, "tabs-1");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.selected), 0);
}

#[gpui::test]
fn arrows_move_focus_and_enter_selects(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { selected: 0 });
    // Tab lands on the selected tab (the single tab stop); Right skips the disabled tab.
    focus_first(cx);
    cx.simulate_keystrokes("right");
    assert_eq!(
        host.read_with(cx, |h, _| h.selected),
        0,
        "focus moves, selection does not"
    );
    tap(cx, "enter");
    assert_eq!(host.read_with(cx, |h, _| h.selected), 2);
}

#[gpui::test]
fn arrows_wrap_around(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { selected: 0 });
    focus_first(cx);
    cx.simulate_keystrokes("left");
    tap(cx, "enter");
    assert_eq!(host.read_with(cx, |h, _| h.selected), 2);
}

#[gpui::test]
fn home_and_end_jump(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { selected: 0 });
    focus_first(cx);
    cx.simulate_keystrokes("end");
    tap(cx, "enter");
    assert_eq!(host.read_with(cx, |h, _| h.selected), 2);
    cx.simulate_keystrokes("home");
    tap(cx, "enter");
    assert_eq!(host.read_with(cx, |h, _| h.selected), 0);
}
