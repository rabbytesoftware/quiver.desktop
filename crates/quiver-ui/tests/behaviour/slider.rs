use gpui::prelude::*;
use gpui::{div, point, px, Context, Modifiers, MouseButton, TestAppContext, Window};
use quiver_ui::Slider;

use crate::{focus_first, open};

struct Host {
    value: f32,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(206.))
            .child(
                Slider::new("sl")
                    .value(self.value)
                    .step(5.)
                    .on_change(cx.processor(|this, v, _, cx| {
                        this.value = v;
                        cx.notify();
                    })),
            )
    }
}

#[gpui::test]
fn pressing_the_track_sets_the_value(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: 0.0 });
    let b = cx.debug_bounds("sl").unwrap();
    cx.simulate_mouse_down(
        point(b.origin.x + b.size.width / 2.0, b.center().y),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_up(b.center(), MouseButton::Left, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value), 50.0);
}

#[gpui::test]
fn dragging_follows_the_pointer_and_stops_on_release(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: 0.0 });
    let b = cx.debug_bounds("sl").unwrap();
    let y = b.center().y;
    cx.simulate_mouse_down(
        point(b.origin.x + px(3.), y),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(host.read_with(cx, |h, _| h.value), 0.0);
    cx.simulate_mouse_move(
        point(b.origin.x + b.size.width * 0.75, y),
        MouseButton::Left,
        Modifiers::none(),
    );
    let dragged = host.read_with(cx, |h, _| h.value);
    assert!(
        (70.0..=80.0).contains(&dragged) && dragged % 5.0 == 0.0,
        "stepped value near 75, got {dragged}"
    );
    cx.simulate_mouse_move(
        point(b.origin.x + b.size.width * 5.0, y),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(
        host.read_with(cx, |h, _| h.value),
        100.0,
        "clamped to max past the end"
    );
    cx.simulate_mouse_up(b.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        point(b.origin.x + px(3.), y),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(
        host.read_with(cx, |h, _| h.value),
        100.0,
        "released: moves no longer drag"
    );
}

#[gpui::test]
fn arrow_keys_and_home_end(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { value: 50.0 });
    focus_first(cx);
    cx.simulate_keystrokes("right right");
    assert_eq!(host.read_with(cx, |h, _| h.value), 60.0);
    cx.simulate_keystrokes("left pageup");
    assert_eq!(host.read_with(cx, |h, _| h.value), 100.0);
    cx.simulate_keystrokes("home");
    assert_eq!(host.read_with(cx, |h, _| h.value), 0.0);
    cx.simulate_keystrokes("end");
    assert_eq!(host.read_with(cx, |h, _| h.value), 100.0);
}
