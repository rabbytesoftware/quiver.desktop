//! `NumberField`: port of `src/components/ui/number-field.tsx` (decrement, centred input,
//! increment, optional suffix). Behaviour follows Base UI: the text is parsed and clamped to
//! `min..=max` on Enter or blur; Up/Down step, Shift = 10x, Alt = 0.1x; PageUp/PageDown = 10x;
//! Home/End jump to min/max. An empty field reports `None`.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    black, div, px, App, ElementId, Entity, Focusable, KeyDownEvent, SharedString, Subscription,
    Window,
};
use gpui_component::input::{Input as Editor, InputEvent, InputState};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::focus::{keyed_container_focus, ring, FocusableExt};
use crate::ids::child_id;
use crate::theme::ActiveQuiver;

type ChangeFn = Rc<dyn Fn(Option<f64>, &mut Window, &mut App)>;

/// Clamps `v` into the optional bounds.
pub(crate) fn clamp(
    v: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> f64 {
    let v = min.map_or(v, |m| v.max(m));
    max.map_or(v, |m| v.min(m))
}

/// The text the field shows for `v`: no trailing `.0`.
pub(crate) fn format_value(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

/// Parses user text; empty or unparsable text is `None`.
pub(crate) fn parse(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The step a key press applies, or `None` for keys that do not step.
pub(crate) fn key_delta(
    key: &str,
    shift: bool,
    alt: bool,
    step: f64,
) -> Option<f64> {
    let scale = if shift {
        10.0
    } else if alt {
        0.1
    } else {
        1.0
    };
    match key {
        "up" => Some(step * scale),
        "down" => Some(-step * scale),
        "pageup" => Some(step * 10.0),
        "pagedown" => Some(-step * 10.0),
        _ => None,
    }
}

struct FieldState {
    input: Entity<InputState>,
    min: Option<f64>,
    max: Option<f64>,
    on_change: Option<ChangeFn>,
    _subscription: Subscription,
}

impl FieldState {
    /// Parses the text, clamps it, rewrites the text and reports the value.
    fn commit(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let parsed =
            parse(self.input.read(cx).value().as_ref()).map(|v| clamp(v, self.min, self.max));
        let text = parsed.map(format_value).unwrap_or_default();
        self.input.update(cx, |i, cx| i.set_value(text, window, cx));
        if let Some(f) = self.on_change.clone() {
            f(parsed, window, cx);
        }
    }

    fn nudge(
        &mut self,
        delta: f64,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let current = parse(self.input.read(cx).value().as_ref()).unwrap_or(0.0);
        let next = clamp(current + delta, self.min, self.max);
        self.input
            .update(cx, |i, cx| i.set_value(format_value(next), window, cx));
        if let Some(f) = self.on_change.clone() {
            f(Some(next), window, cx);
        }
    }
}

#[derive(IntoElement)]
pub struct NumberField {
    id: ElementId,
    value: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    suffix: Option<SharedString>,
    disabled: bool,
    on_change: Option<ChangeFn>,
}

impl NumberField {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            min: None,
            max: None,
            step: 1.0,
            suffix: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn value(
        mut self,
        v: Option<f64>,
    ) -> Self {
        self.value = v;
        self
    }

    pub fn range(
        mut self,
        min: f64,
        max: f64,
    ) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn step(
        mut self,
        step: f64,
    ) -> Self {
        self.step = step;
        self
    }

    /// Unit text after the field (`ms`, `%`).
    pub fn suffix(
        mut self,
        suffix: impl Into<SharedString>,
    ) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        f: impl Fn(Option<f64>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for NumberField {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let initial = self.value.map(format_value).unwrap_or_default();
        let state = window.use_keyed_state(child_id(&self.id, "state"), cx, move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).default_value(initial));
            let sub = cx.subscribe_in(
                &input,
                window,
                |this: &mut FieldState, _, ev: &InputEvent, window, cx| {
                    if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                        this.commit(window, cx);
                    }
                },
            );
            FieldState {
                input,
                min: None,
                max: None,
                on_change: None,
                _subscription: sub,
            }
        });
        let (min, max, step) = (self.min, self.max, self.step);
        let input = state.update(cx, |s, _| {
            s.min = min;
            s.max = max;
            s.on_change = self.on_change.clone();
            s.input.clone()
        });
        // Controlled value: mirror the prop unless the user is typing.
        let shown = self.value.map(format_value).unwrap_or_default();
        let typing = input.read(cx).focus_handle(cx).contains_focused(window, cx);
        if !typing && input.read(cx).value().as_ref() != shown {
            input.update(cx, |i, cx| i.set_value(shown, window, cx));
        }

        let within = keyed_container_focus(&self.id, window, cx);
        let current = self.value.unwrap_or(0.0);
        let can_dec = !self.disabled && min.is_none_or(|m| current > m);
        let can_inc = !self.disabled && max.is_none_or(|m| current < m);
        let step_button = |name: &'static str, label: &'static str, delta: f64, enabled: bool| {
            let st = state.clone();
            let key = format!("{}-{name}", self.id);
            div()
                .id(child_id(&self.id, name))
                .debug_selector(move || key)
                .flex()
                .items_center()
                .justify_center()
                .w(px(24.))
                .h_full()
                .text_color(p.muted_foreground)
                .child(label)
                .when(!enabled, |d| d.opacity(0.5))
                .when(enabled, |d| {
                    d.cursor_pointer()
                        .hover(move |s| s.bg(p.accent).text_color(p.accent_foreground))
                        .no_pointer_focus()
                        .on_click(move |_, window, cx| {
                            st.update(cx, |s, cx| s.nudge(delta, window, cx))
                        })
                })
        };

        let st_keys = state.clone();
        let key = self.id.to_string();
        let ring_color = p.ring.opacity(0.5);
        let ring_border = p.ring;
        let group = div()
            .id(self.id.clone())
            .debug_selector(move || key)
            .track_focus(&within)
            .flex()
            .items_center()
            .h(px(32.))
            .overflow_hidden()
            .rounded(px(Radius::MD))
            .border_1()
            .border_color(p.input)
            .shadow(shadow_xs(black().opacity(0.05)))
            .when(self.disabled, |d| d.opacity(0.5))
            .in_focus(move |mut s| {
                s.border_color = Some(ring_border);
                s.box_shadow = Some(vec![ring(ring_color, 3.0)]);
                s
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let m = e.keystroke.modifiers;
                let key = e.keystroke.key.as_str();
                let target = match key {
                    "home" => min.map(|v| v - parse_current(&st_keys, cx)),
                    "end" => max.map(|v| v - parse_current(&st_keys, cx)),
                    _ => key_delta(key, m.shift, m.alt, step),
                };
                if let Some(delta) = target {
                    st_keys.update(cx, |s, cx| s.nudge(delta, window, cx));
                    cx.stop_propagation();
                }
            })
            .child(step_button("dec", "\u{2212}", -step, can_dec))
            .child(
                div()
                    .w(px(56.))
                    .h_full()
                    .text_size(px(14.))
                    .text_color(p.foreground)
                    .child(
                        Editor::new(&input)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .disabled(self.disabled)
                            .h_full(),
                    ),
            )
            .child(step_button("inc", "+", step, can_inc));
        div()
            .flex()
            .items_center()
            .child(group)
            .children(self.suffix.map(|s| {
                div()
                    .ml(px(6.))
                    .text_size(px(12.))
                    .text_color(p.muted_foreground)
                    .child(s)
            }))
    }
}

fn parse_current(
    state: &Entity<FieldState>,
    cx: &App,
) -> f64 {
    let s = state.read(cx);
    parse(s.input.read(cx).value().as_ref()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_respects_both_bounds() {
        assert_eq!(clamp(-5.0, Some(0.0), Some(100.0)), 0.0);
        assert_eq!(clamp(500.0, Some(0.0), Some(100.0)), 100.0);
        assert_eq!(clamp(50.0, None, None), 50.0);
    }

    #[test]
    fn formatting_drops_the_fraction_of_whole_numbers() {
        assert_eq!(format_value(50.0), "50");
        assert_eq!(format_value(0.5), "0.5");
    }

    #[test]
    fn empty_and_garbage_are_none() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse(" 12 "), Some(12.0));
    }

    #[test]
    fn modifier_scales_the_step() {
        assert_eq!(key_delta("up", false, false, 5.0), Some(5.0));
        assert_eq!(key_delta("up", true, false, 5.0), Some(50.0));
        assert_eq!(key_delta("down", false, true, 5.0), Some(-0.5));
        assert_eq!(key_delta("pageup", false, false, 5.0), Some(50.0));
        assert_eq!(key_delta("a", false, false, 5.0), None);
    }
}
