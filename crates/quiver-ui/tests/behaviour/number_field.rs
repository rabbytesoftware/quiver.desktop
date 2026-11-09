use gpui::prelude::*;
use gpui::{div, Context, Modifiers, TestAppContext, Window};
use quiver_ui::NumberField;

use crate::{centre, open};

struct Host {
    value: Option<f64>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            NumberField::new("num")
                .value(self.value)
                .range(0., 100.)
                .step(5.)
                .on_change(cx.processor(|this, v, _, cx| {
                    this.value = v;
                    cx.notify();
                })),
        )
    }
}

#[gpui::test]
fn buttons_step_by_the_step(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: Some(50.0) });
    let inc = centre(cx, "num-inc");
    cx.simulate_click(inc, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(55.0));
    let dec = centre(cx, "num-dec");
    cx.simulate_click(dec, Modifiers::none());
    cx.simulate_click(dec, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(45.0));
}

#[gpui::test]
fn increment_clamps_at_max_and_decrement_at_min(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: Some(98.0) });
    let inc = centre(cx, "num-inc");
    cx.simulate_click(inc, Modifiers::none());
    assert_eq!(
        host.read_with(cx, |h, _| h.value),
        Some(100.0),
        "98 + 5 clamps to the max"
    );
    cx.simulate_click(inc, Modifiers::none());
    assert_eq!(
        host.read_with(cx, |h, _| h.value),
        Some(100.0),
        "already at max: the button is inert"
    );

    let (host, cx) = open(cx, |_, _| Host { value: Some(2.0) });
    let dec = centre(cx, "num-dec");
    cx.simulate_click(dec, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(0.0));
}

#[gpui::test]
fn typed_text_is_clamped_on_enter(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: Some(10.0) });
    let field = centre(cx, "num");
    cx.simulate_click(field, Modifiers::none());
    cx.simulate_keystrokes("ctrl-a");
    cx.simulate_input("999");
    cx.simulate_keystrokes("enter");
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(100.0));
}

#[gpui::test]
fn arrow_up_steps_from_the_keyboard(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: Some(10.0) });
    let field = centre(cx, "num");
    cx.simulate_click(field, Modifiers::none());
    cx.simulate_keystrokes("up up");
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(20.0));
    cx.simulate_keystrokes("shift-up");
    assert_eq!(host.read_with(cx, |h, _| h.value), Some(70.0));
}
