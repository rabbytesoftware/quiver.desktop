use gpui::prelude::*;
use gpui::{div, px, Context, Modifiers, SharedString, TestAppContext, Window};
use quiver_ui::{Select, SelectOption};

use crate::{centre, focus_first, open, tap};

struct Host {
    value: Option<SharedString>,
    open: bool,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().w(px(200.)).child(
            Select::new("sel")
                .options(vec![
                    SelectOption::new("a", "Alpha"),
                    SelectOption::new("b", "Beta").disabled(true),
                    SelectOption::new("c", "Gamma"),
                ])
                .value(self.value.clone())
                .on_open_change(cx.processor(|this, open, _, _| this.open = open))
                .on_change(cx.processor(|this, v, _, cx| {
                    this.value = Some(v);
                    cx.notify();
                })),
        )
    }
}

#[gpui::test]
fn click_opens_and_picking_an_option_selects_and_closes(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host {
        value: None,
        open: false,
    });
    assert!(!host.read_with(cx, |h, _| h.open), "closed at first");
    let trigger = centre(cx, "sel");
    cx.simulate_click(trigger, Modifiers::none());
    assert!(host.read_with(cx, |h, _| h.open), "open after a click");
    assert!(
        cx.debug_bounds("sel-option-0").is_some(),
        "the options are laid out"
    );
    let option = centre(cx, "sel-option-2");
    cx.simulate_click(option, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value.clone()), Some("c".into()));
    assert!(!host.read_with(cx, |h, _| h.open), "closed after choosing");
}

#[gpui::test]
fn clicking_the_trigger_again_closes(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host {
        value: None,
        open: false,
    });
    let trigger = centre(cx, "sel");
    cx.simulate_click(trigger, Modifiers::none());
    cx.simulate_click(trigger, Modifiers::none());
    assert!(!host.read_with(cx, |h, _| h.open));
}

#[gpui::test]
fn escape_closes_without_choosing(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host {
        value: None,
        open: false,
    });
    focus_first(cx);
    cx.simulate_keystrokes("down");
    assert!(host.read_with(cx, |h, _| h.open), "Down opens");
    cx.simulate_keystrokes("escape");
    assert!(!host.read_with(cx, |h, _| h.open));
    assert_eq!(host.read_with(cx, |h, _| h.value.clone()), None);
}

#[gpui::test]
fn keyboard_skips_disabled_options_and_enter_chooses(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host {
        value: None,
        open: false,
    });
    focus_first(cx);
    cx.simulate_keystrokes("down down down");
    tap(cx, "enter");
    // Down opens with nothing lit; then Alpha, (Beta is disabled) Gamma.
    assert_eq!(host.read_with(cx, |h, _| h.value.clone()), Some("c".into()));
}

#[gpui::test]
fn disabled_option_cannot_be_clicked(cx: &mut TestAppContext) {
    let (host, cx) = open(cx, |_, _| Host {
        value: None,
        open: false,
    });
    let trigger = centre(cx, "sel");
    cx.simulate_click(trigger, Modifiers::none());
    let option = centre(cx, "sel-option-1");
    cx.simulate_click(option, Modifiers::none());
    assert_eq!(host.read_with(cx, |h, _| h.value.clone()), None);
}
