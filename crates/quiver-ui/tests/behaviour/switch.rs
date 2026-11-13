use gpui::prelude::*;
use gpui::{div, Context, Modifiers, TestAppContext, Window};
use quiver_ui::Switch;

use crate::{centre, focus_first, open, tap};

struct Host {
    on: bool,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().child(
            Switch::new("sw")
                .checked(self.on)
                .on_change(cx.processor(|this, v, _, cx| {
                    this.on = v;
                    cx.notify();
                })),
        )
    }
}

#[gpui::test]
fn click_toggles(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { on: false });
    let at = centre(cx, "sw");
    cx.simulate_click(at, Modifiers::none());
    assert!(host.read_with(cx, |h, _| h.on));
    cx.simulate_click(at, Modifiers::none());
    assert!(!host.read_with(cx, |h, _| h.on));
}

#[gpui::test]
fn space_toggles_but_enter_does_not(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { on: false });
    focus_first(cx);
    tap(cx, "enter");
    assert!(!host.read_with(cx, |h, _| h.on));
    tap(cx, "space");
    assert!(host.read_with(cx, |h, _| h.on));
}
