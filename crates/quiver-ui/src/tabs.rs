//! `Tabs`: port of `src/components/ui/tabs.tsx` (`default` pill list and `underline` variants,
//! horizontal and vertical).
//!
//! Base UI semantics: one tab stop (the selected tab); arrows move focus with wrap, Home/End jump;
//! Enter or Space selects the focused tab (selection does not follow focus).
//! The sliding indicator is replaced by painting the selected surface on the active tab: the
//! original disables its transition anyway ("snapping instantly is the reliable option").

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    black, div, px, transparent_black, AnyElement, App, ClickEvent, ElementId, FocusHandle,
    FontWeight, KeyDownEvent, SharedString, StyleRefinement, Window,
};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::focus::{keyed_focus, FocusableExt};
use crate::ids::child_id;
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsVariant {
    #[default]
    Default,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

pub struct TabItem {
    label: SharedString,
    icon: Option<AnyElement>,
    disabled: bool,
}

impl TabItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(icon.into_any_element());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The index reached from `from` going `dir` (+1/-1), skipping disabled tabs, wrapping around.
pub(crate) fn step_enabled(from: usize, dir: isize, disabled: &[bool]) -> Option<usize> {
    let n = disabled.len() as isize;
    (1..=n)
        .map(|k| (from as isize + dir * k).rem_euclid(n) as usize)
        .find(|i| !disabled[*i])
}

type ChangeFn = Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    tabs: Vec<TabItem>,
    selected: usize,
    variant: TabsVariant,
    orientation: Orientation,
    on_change: Option<ChangeFn>,
    panel: Option<AnyElement>,
    style: StyleRefinement,
}

impl Tabs {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected: 0,
            variant: TabsVariant::Default,
            orientation: Orientation::Horizontal,
            on_change: None,
            panel: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn tab(mut self, tab: TabItem) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn variant(mut self, v: TabsVariant) -> Self {
        self.variant = v;
        self
    }

    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }

    pub fn on_change(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// The panel of the selected tab (`TabsPanel`).
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }
}

impl Styled for Tabs {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let q = *cx.quiver();
        let p = q.palette;
        let vertical = self.orientation == Orientation::Vertical;
        let disabled: Vec<bool> = self.tabs.iter().map(|t| t.disabled).collect();
        let handles: Vec<FocusHandle> = (0..self.tabs.len())
            .map(|i| keyed_focus(&child_id(&self.id, i.to_string()), window, cx))
            .collect();
        let underline = self.variant == TabsVariant::Underline;
        let list_fg = if underline {
            p.muted_foreground
        } else {
            p.muted_foreground.opacity(0.72)
        };
        let selected = self.selected;
        let on_change = self.on_change;

        let tabs: Vec<AnyElement> = self
            .tabs
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let active = i == selected;
                let tab_id = child_id(&self.id, i.to_string());
                let key = format!("{}-{}", self.id, i);
                let (handles, disabled, on_change) =
                    (handles.clone(), disabled.clone(), on_change.clone());
                let focus = handles[i].clone().tab_stop(active);
                let accent = p.accent;
                let ring_color = p.ring;
                let fg = if active && !underline {
                    p.background
                } else if active {
                    p.foreground
                } else {
                    list_fg
                };
                let mut el = div()
                    .id(tab_id)
                    .debug_selector(move || key)
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .when(vertical, |d| d.w_full().justify_start())
                    .when(!vertical, |d| d.justify_center())
                    .gap(px(6.))
                    .h(px(32.))
                    .px(px(9.))
                    .border_1()
                    .border_color(transparent_black())
                    .rounded(px(if active && !underline {
                        Radius::LG
                    } else {
                        Radius::MD
                    }))
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .text_color(fg)
                    .when(active && !underline, |d| {
                        d.bg(p.foreground)
                            .border_color(p.foreground)
                            .shadow(shadow_xs(black().opacity(0.10)))
                    })
                    .when(active && underline, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom(px(-5.))
                                .h(px(2.))
                                .bg(p.primary),
                        )
                    })
                    .children(t.icon)
                    .child(t.label);
                if t.disabled {
                    return el.opacity(0.64).into_any_element();
                }
                el = el
                    .cursor_pointer()
                    .track_focus(&focus)
                    .no_pointer_focus()
                    .when(!active, |d| {
                        d.hover(move |s| {
                            if underline {
                                s.bg(accent)
                            } else {
                                s.text_color(p.muted_foreground)
                            }
                        })
                    })
                    .focus(move |mut s| {
                        s.box_shadow = Some(vec![crate::focus::ring(ring_color, 2.0)]);
                        s
                    })
                    .on_click({
                        let on_change = on_change.clone();
                        move |_: &ClickEvent, window, cx| {
                            if let Some(f) = &on_change {
                                f(i, window, cx);
                            }
                        }
                    })
                    .on_key_down(move |e: &KeyDownEvent, window, cx| {
                        let (prev, next) = if vertical {
                            ("up", "down")
                        } else {
                            ("left", "right")
                        };
                        let target = match e.keystroke.key.as_str() {
                            k if k == next => step_enabled(i, 1, &disabled),
                            k if k == prev => step_enabled(i, -1, &disabled),
                            "home" => step_enabled(disabled.len() - 1, 1, &disabled),
                            "end" => step_enabled(0, -1, &disabled),
                            _ => None,
                        };
                        if let Some(t) = target {
                            window.focus(&handles[t]);
                            cx.stop_propagation();
                        }
                    });
                el.into_any_element()
            })
            .collect();

        let list = div()
            .relative()
            .flex()
            .when(vertical, |d| d.flex_col())
            .items_center()
            .gap(px(2.))
            .w_auto()
            .text_color(list_fg)
            .when(!underline, |d| {
                d.rounded(px(Radius::LG)).bg(p.muted).p(px(2.))
            })
            .when(underline && !vertical, |d| d.py(px(4.)))
            .when(underline && vertical, |d| d.px(px(4.)))
            .children(tabs);
        let mut el = div()
            .flex()
            .gap(px(8.))
            .when(!vertical, |d| d.flex_col())
            .child(div().flex().child(list))
            .children(self.panel.map(|panel| div().flex_1().child(panel)));
        el.style().refine(&self.style);
        el
    }
}
