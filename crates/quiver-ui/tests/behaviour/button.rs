use gpui::prelude::*;
use gpui::{div, px, Context, Modifiers, TestAppContext, Window};
use quiver_ui::{Button, ButtonVariant};

use crate::{centre, focus_first, open, tap};

struct Host {
    clicks: Vec<&'static str>,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let on = |name: &'static str| {
            cx.listener(move |this: &mut Host, _, _, cx| {
                this.clicks.push(name);
                cx.notify();
            })
        };
        div()
            .flex()
            .gap(px(8.))
            .child(Button::new("first").label("First").on_click(on("first")))
            .child(
                Button::new("second")
                    .variant(ButtonVariant::Outline)
                    .label("Second")
                    .on_click(on("second")),
            )
            .child(
                Button::new("off")
                    .disabled(true)
                    .label("Off")
                    .on_click(on("off")),
            )
            .child(
                Button::new("busy")
                    .loading(true)
                    .label("Busy")
                    .on_click(on("busy")),
            )
    }
}

#[gpui::test]
fn click_fires_the_handler(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { clicks: vec![] });
    let at = centre(cx, "first");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.clicks.clone()), vec!["first"]);
}

#[gpui::test]
fn disabled_and_loading_buttons_ignore_clicks(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { clicks: vec![] });
    for id in ["off", "busy"] {
        let at = centre(cx, id);
        cx.simulate_click(at, Modifiers::none());
    }
    assert!(host.read_with(cx, |h, _| h.clicks.is_empty()));
}

#[gpui::test]
fn tab_walks_the_tab_stops_and_enter_or_space_clicks(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { clicks: vec![] });
    focus_first(cx);
    tap(cx, "enter");
    cx.simulate_keystrokes("tab");
    tap(cx, "space");
    assert_eq!(
        host.read_with(cx, |h, _| h.clicks.clone()),
        vec!["first", "second"]
    );
}

#[gpui::test]
fn disabled_buttons_are_skipped_by_tab(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host { clicks: vec![] });
    focus_first(cx);
    cx.simulate_keystrokes("tab tab");
    tap(cx, "enter");
    // first, second, then wraps past the two inert buttons back to first.
    assert_eq!(host.read_with(cx, |h, _| h.clicks.clone()), vec!["first"]);
}
