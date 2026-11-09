//! Behaviour tests: every interactive component driven through real GPUI input events
//! (mouse clicks, keystrokes) in a test window, no sleeps.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod button;
mod number_field;
mod select;
mod slider;
mod switch;
mod tabs;

use gpui::{AppContext, Entity, TestAppContext, VisualTestContext};
use gpui_component::Root;
use quiver_theme::Mode;

/// Opens a test window whose root is gpui-component's `Root` around `build`'s view.
pub fn open<V: gpui::Render + 'static>(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut gpui::Window, &mut gpui::Context<V>) -> V + 'static,
) -> (Entity<V>, &mut VisualTestContext) {
    cx.update(|cx| quiver_ui::install(cx, Mode::Light));
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<V>>>> = Default::default();
    let out = slot.clone();
    let (_, vcx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| build(window, cx));
        *out.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = slot.borrow_mut().take().expect("view built");
    (view, vcx)
}

/// The centre of the element registered under `selector`.
pub fn centre(cx: &mut VisualTestContext, selector: &'static str) -> gpui::Point<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("no element `{selector}` was rendered"))
        .center()
}

/// Moves focus to the first tab stop, as the app does on startup. (Tab only works once something
/// inside the `Root` has focus.)
pub fn focus_first(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.focus_next());
    cx.run_until_parked();
    assert!(
        cx.update(|window, cx| window.focused(cx).is_some()),
        "no tab stop took focus"
    );
}

/// Presses and releases `key`. GPUI fires keyboard clicks (Enter/Space on a focused button) on key
/// *up*, which `simulate_keystrokes` does not send.
pub fn tap(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    let keystroke = gpui::Keystroke::parse(key).expect("valid key");
    cx.simulate_event(gpui::KeyUpEvent { keystroke });
}
