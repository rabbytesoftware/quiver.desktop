//! `Slider`: port of `src/components/ui/slider.tsx` (single horizontal thumb, `thumbAlignment="edge"`).
//! Drag, track click, arrows (+-step), PageUp/PageDown (+-10 steps), Home/End.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    canvas, div, px, App, Bounds, ElementId, Hsla, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, StyleRefinement, Window,
};

use crate::focus::{keyed_focus, ring, FocusableExt};
use crate::theme::ActiveQuiver;

const THUMB_W: f32 = 6.0;
const THUMB_H: f32 = 16.0;
const TRACK_H: f32 = 6.0;

/// Shared between frames: where the control was last laid out and whether a drag is running.
#[derive(Default)]
struct DragState {
    bounds: Bounds<Pixels>,
    dragging: bool,
}

/// Maps a pointer x (window space) inside `bounds` to a stepped value in `min..=max`.
pub(crate) fn value_at(x: f32, left: f32, width: f32, min: f32, max: f32, step: f32) -> f32 {
    let travel = (width - THUMB_W).max(1.0);
    let t = ((x - left - THUMB_W / 2.0) / travel).clamp(0.0, 1.0);
    snap(min + t * (max - min), min, max, step)
}

pub(crate) fn snap(v: f32, min: f32, max: f32, step: f32) -> f32 {
    if step <= 0.0 {
        return v.clamp(min, max);
    }
    (min + ((v - min) / step).round() * step).clamp(min, max)
}

type ChangeFn = Rc<dyn Fn(f32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    on_change: Option<ChangeFn>,
    style: StyleRefinement,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            disabled: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn value(mut self, v: f32) -> Self {
        self.value = v;
        self
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, f: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn key_value(key: &str, v: f32, min: f32, max: f32, step: f32) -> Option<f32> {
    let big = step * 10.0;
    let next = match key {
        "right" | "up" => v + step,
        "left" | "down" => v - step,
        "pageup" => v + big,
        "pagedown" => v - big,
        "home" => min,
        "end" => max,
        _ => return None,
    };
    Some(snap(next, min, max, step))
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.quiver().palette;
        let state =
            window.use_keyed_state(ElementId::from((self.id.clone(), "drag")), cx, |_, _| {
                DragState::default()
            });
        let focus = keyed_focus(&self.id, window, cx);
        let (min, max, step) = (self.min, self.max, self.step);
        let fraction = if max > min {
            ((self.value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bounds = state.read(cx).bounds;
        let travel = (f32::from(bounds.size.width) - THUMB_W).max(0.0);
        let thumb_left = fraction * travel;
        let fill_w = thumb_left + THUMB_W / 2.0;
        let key = self.id.to_string();
        let thumb_ring = ring(p.ring.opacity(0.5), 4.0);
        let thumb_hover = thumb_ring.clone();
        let disabled = self.disabled;
        let report = {
            let on_change = self.on_change.clone();
            move |v: f32, window: &mut Window, cx: &mut App| {
                if let Some(f) = &on_change {
                    f(v, window, cx);
                }
            }
        };
        let (down_state, down_report) = (state.clone(), report.clone());
        let (key_value_now, key_report) = (self.value, report.clone());
        let mut el = div()
            .id(self.id)
            .debug_selector(move || key)
            .relative()
            .w_full()
            .h(px(THUMB_H))
            .flex()
            .items_center()
            .when(disabled, |d| d.opacity(0.5))
            .child(
                canvas(
                    {
                        let state = state.clone();
                        move |b, _, cx| state.update(cx, |s, _| s.bounds = b)
                    },
                    {
                        let state = state.clone();
                        let report = report.clone();
                        move |_, _, window: &mut Window, _: &mut App| {
                            let (s2, r2) = (state.clone(), report.clone());
                            window.on_mouse_event(move |e: &MouseMoveEvent, _, window, cx| {
                                let (dragging, b) = {
                                    let s = s2.read(cx);
                                    (s.dragging, s.bounds)
                                };
                                if dragging {
                                    r2(
                                        value_at(
                                            f32::from(e.position.x),
                                            f32::from(b.origin.x),
                                            f32::from(b.size.width),
                                            min,
                                            max,
                                            step,
                                        ),
                                        window,
                                        cx,
                                    );
                                }
                            });
                            let s3 = state.clone();
                            window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                                s3.update(cx, |s, _| s.dragging = false)
                            });
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .w_full()
                    .h(px(TRACK_H))
                    .overflow_hidden()
                    .bg(p.muted_foreground.opacity(0.3))
                    .child(div().h_full().w(px(fill_w)).bg(p.primary)),
            )
            .child(thumb(
                p.primary,
                thumb_left,
                &focus,
                thumb_ring,
                thumb_hover,
                disabled,
                min,
                max,
                step,
                key_value_now,
                key_report,
            ));
        if !disabled {
            el = el.cursor_pointer().no_pointer_focus().on_mouse_down(
                MouseButton::Left,
                move |e: &MouseDownEvent, window, cx| {
                    let b = down_state.update(cx, |s, _| {
                        s.dragging = true;
                        s.bounds
                    });
                    down_report(
                        value_at(
                            f32::from(e.position.x),
                            f32::from(b.origin.x),
                            f32::from(b.size.width),
                            min,
                            max,
                            step,
                        ),
                        window,
                        cx,
                    );
                },
            );
        }
        el.style().refine(&self.style);
        el
    }
}

#[allow(clippy::too_many_arguments)]
fn thumb(
    color: Hsla,
    left: f32,
    focus: &gpui::FocusHandle,
    focus_ring: gpui::BoxShadow,
    hover_ring: gpui::BoxShadow,
    disabled: bool,
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    report: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id("thumb")
        .absolute()
        .top(px(0.))
        .left(px(left))
        .w(px(THUMB_W))
        .h(px(THUMB_H))
        .border_1()
        .border_color(color)
        .bg(color)
        .shadow_sm()
        .when(!disabled, |d| {
            d.track_focus(focus)
                .tab_stop(true)
                .hover(move |mut s| {
                    s.box_shadow = Some(vec![hover_ring]);
                    s
                })
                .focus(move |mut s| {
                    s.box_shadow = Some(vec![focus_ring]);
                    s
                })
                .on_key_down(move |e: &KeyDownEvent, window, cx| {
                    if let Some(v) = key_value(&e.keystroke.key, value, min, max, step) {
                        report(v, window, cx);
                        cx.stop_propagation();
                    }
                })
        })
}
