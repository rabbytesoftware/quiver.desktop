//! `Select`: port of `src/components/ui/select.tsx` (trigger button, popup list with a check
//! indicator; sizes `sm`, `default`, `lg`).
//! Trigger: Enter/Space/Down/Up open. Popup: Down/Up/Home/End move, Enter/Space choose, Escape or an
//! outside press close. Item-aligned popup positioning (`alignItemWithTrigger`) is not ported: the
//! popup opens under the trigger. The scroll arrows are replaced by a scrollable list.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    black, div, px, App, ElementId, KeyDownEvent, MouseDownEvent, MouseMoveEvent, SharedString,
    StyleRefinement, Window,
};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::focus::{ring, FocusableExt};
use crate::icon::{icon, Glyph};
use crate::ids::child_id;
use crate::popup::{floating, nav_key, popup_surface, set_open, use_popup, Nav, OpenFn};
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectSize {
    Sm,
    #[default]
    Default,
    Lg,
}

#[derive(Clone)]
pub struct SelectOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }
}

type ChangeFn = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    options: Vec<SelectOption>,
    value: Option<SharedString>,
    placeholder: SharedString,
    size: SelectSize,
    disabled: bool,
    invalid: bool,
    on_change: Option<ChangeFn>,
    on_open_change: Option<OpenFn>,
    style: StyleRefinement,
}

impl Select {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            value: None,
            placeholder: SharedString::default(),
            size: SelectSize::Default,
            disabled: false,
            invalid: false,
            on_change: None,
            on_open_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn options(
        mut self,
        options: Vec<SelectOption>,
    ) -> Self {
        self.options = options;
        self
    }

    pub fn value(
        mut self,
        value: Option<SharedString>,
    ) -> Self {
        self.value = value;
        self
    }

    pub fn placeholder(
        mut self,
        text: impl Into<SharedString>,
    ) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn size(
        mut self,
        size: SelectSize,
    ) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }

    /// `aria-invalid`: destructive border.
    pub fn invalid(
        mut self,
        invalid: bool,
    ) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn on_change(
        mut self,
        f: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Called when the popup opens or closes.
    pub fn on_open_change(
        mut self,
        f: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Select {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Select {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let q = *cx.quiver();
        let p = q.palette;
        let trigger_id = child_id(&self.id, "trigger");
        let h = use_popup(&self.id, &trigger_id, window, cx);
        let (open, highlighted) = {
            let s = h.state.read(cx);
            (s.open, s.highlighted)
        };
        let (min_h, pad, gap) = match self.size {
            SelectSize::Sm => (28.0, 9.0, 6.0),
            SelectSize::Default => (32.0, 11.0, 8.0),
            SelectSize::Lg => (36.0, 11.0, 8.0),
        };
        let selected_index = self
            .value
            .as_ref()
            .and_then(|v| self.options.iter().position(|o| &o.value == v));
        let disabled_flags: Vec<bool> = self.options.iter().map(|o| o.disabled).collect();
        let values: Vec<SharedString> = self.options.iter().map(|o| o.value.clone()).collect();
        let on_change = self.on_change.clone();
        let shown = selected_index.map(|i| self.options[i].label.clone());
        let invalid = self.invalid;
        let ring_color = if invalid {
            p.destructive.opacity(0.16)
        } else {
            p.ring.opacity(0.24)
        };
        let focus_border = if invalid {
            p.destructive.opacity(0.64)
        } else {
            p.ring
        };
        let key = self.id.to_string();

        let (st_click, pf_click) = (h.state.clone(), h.popup_focus.clone());
        let (st_press, st_keys, pf_keys) =
            (h.state.clone(), h.state.clone(), h.popup_focus.clone());
        let (oo_click, oo_keys) = (self.on_open_change.clone(), self.on_open_change.clone());
        let initial = selected_index;
        let mut trigger = div()
            .id(trigger_id)
            .debug_selector(move || key)
            .relative()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(gap))
            .w_full()
            .min_w(px(144.))
            .min_h(px(min_h))
            .px(px(pad))
            .rounded(px(Radius::LG))
            .border_1()
            .border_color(if invalid {
                p.destructive.opacity(0.36)
            } else {
                p.input
            })
            .bg(if q.is_dark() {
                p.input.opacity(0.32)
            } else {
                p.background
            })
            .text_size(px(14.))
            .text_color(p.foreground)
            .shadow(shadow_xs(black().opacity(0.05)))
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_color(if shown.is_some() {
                        p.foreground
                    } else {
                        p.muted_foreground.opacity(0.72)
                    })
                    .child(shown.unwrap_or(self.placeholder)),
            )
            .child(div().opacity(0.8).mr(px(-4.)).child(icon(
                Glyph::ChevronsUpDown,
                16.,
                p.foreground,
            )));
        trigger = if self.disabled {
            trigger.opacity(0.64)
        } else {
            trigger
                .cursor_pointer()
                .track_focus(&h.trigger_focus)
                .tab_stop(true)
                .no_pointer_focus()
                .focus(move |mut s| {
                    s.border_color = Some(focus_border);
                    s.box_shadow = Some(vec![ring(ring_color, 3.0)]);
                    s
                })
                .capture_any_mouse_down(move |_: &MouseDownEvent, _, cx| {
                    st_press.update(cx, |s, _| s.was_open_at_press = s.open)
                })
                .on_click(move |e, window, cx| {
                    let keyboard = matches!(e, gpui::ClickEvent::Keyboard(_));
                    if keyboard && st_click.update(cx, |s, _| std::mem::take(&mut s.swallow_click))
                    {
                        return;
                    }
                    let was_open = {
                        let s = st_click.read(cx);
                        if keyboard {
                            s.open
                        } else {
                            s.was_open_at_press
                        }
                    };
                    set_open(&st_click, !was_open, initial, &oo_click, window, cx);
                    if !was_open {
                        window.focus(&pf_click);
                    }
                })
                .on_key_down(move |e: &KeyDownEvent, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "down" | "up") {
                        set_open(&st_keys, true, initial, &oo_keys, window, cx);
                        window.focus(&pf_keys);
                        cx.stop_propagation();
                    }
                })
        };

        let popup = open.then(|| {
            let (st_key, tf_key, st_out) =
                (h.state.clone(), h.trigger_focus.clone(), h.state.clone());
            let (oo_out, oo_key) = (self.on_open_change.clone(), self.on_open_change.clone());
            let (nd, vals, oc) = (disabled_flags.clone(), values.clone(), on_change.clone());
            let rows: Vec<gpui::AnyElement> = self
                .options
                .iter()
                .enumerate()
                .map(|(i, o)| {
                    let (st_hover, st_pick, tf_pick, oc_pick) = (
                        h.state.clone(),
                        h.state.clone(),
                        h.trigger_focus.clone(),
                        on_change.clone(),
                    );
                    let oo_pick = self.on_open_change.clone();
                    let value = o.value.clone();
                    let lit = highlighted == Some(i);
                    let chosen = selected_index == Some(i);
                    let key = format!("{}-option-{}", self.id, i);
                    let row = div()
                        .id(child_id(&self.id, format!("option-{i}")))
                        .debug_selector(move || key)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .min_h(px(28.))
                        .rounded(px(Radius::SM - 2.))
                        .py(px(4.))
                        .pl(px(8.))
                        .pr(px(16.))
                        .text_size(px(14.))
                        .when(lit, |d| d.bg(p.accent).text_color(p.accent_foreground))
                        .child(div().w(px(16.)).flex_none().children(chosen.then(|| {
                            icon(
                                Glyph::Check,
                                16.,
                                if lit {
                                    p.accent_foreground
                                } else {
                                    p.popover_foreground
                                },
                            )
                        })))
                        .child(div().min_w_0().child(o.label.clone()));
                    if o.disabled {
                        return row.opacity(0.64).into_any_element();
                    }
                    row.cursor_pointer()
                        .on_mouse_move(move |_: &MouseMoveEvent, _, cx| {
                            st_hover.update(cx, |s, cx| {
                                if s.highlighted != Some(i) {
                                    s.highlighted = Some(i);
                                    cx.notify();
                                }
                            })
                        })
                        .on_click(move |_, window, cx| {
                            set_open(&st_pick, false, None, &oo_pick, window, cx);
                            window.focus(&tf_pick);
                            if let Some(f) = &oc_pick {
                                f(value.clone(), window, cx);
                            }
                        })
                        .into_any_element()
                })
                .collect();
            popup_surface(cx)
                .id(child_id(&self.id, "popup"))
                .w_full()
                .track_focus(&h.popup_focus)
                .on_mouse_down_out(move |_, window, cx| {
                    set_open(&st_out, false, None, &oo_out, window, cx)
                })
                .on_key_down(move |e: &KeyDownEvent, window, cx| {
                    let cur = st_key.read(cx).highlighted;
                    match nav_key(&e.keystroke.key, cur, &nd) {
                        Nav::Highlight(i) => st_key.update(cx, |s, cx| {
                            s.highlighted = Some(i);
                            cx.notify();
                        }),
                        Nav::Activate => {
                            let Some(i) = cur else { return };
                            st_key.update(cx, |s, _| s.swallow_click = true);
                            set_open(&st_key, false, None, &oo_key, window, cx);
                            window.focus(&tf_key);
                            if let Some(f) = &oc {
                                f(vals[i].clone(), window, cx);
                            }
                        }
                        Nav::Close => {
                            set_open(&st_key, false, None, &oo_key, window, cx);
                            window.focus(&tf_key);
                        }
                        Nav::Ignore => return,
                    }
                    cx.stop_propagation();
                })
                .child(
                    div()
                        .id(child_id(&self.id, "list"))
                        .max_h(px(256.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .children(rows),
                )
                .into_any_element()
        });
        let mut el = floating(trigger.into_any_element(), popup, false);
        el.style().refine(&self.style);
        el
    }
}
