//! `Menu`: port of `src/components/ui/menu.tsx` (trigger + popup with items and separators).
//! Down/Up/Home/End move the highlight, Enter/Space select, Escape closes, outside press closes.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, KeyDownEvent, MouseDownEvent, MouseMoveEvent,
    SharedString, Window,
};
use quiver_theme::Radius;

use crate::button::Button;
use crate::inspect::{probe, Kind};

use crate::ids::child_id;
use crate::popup::{floating, nav_key, popup_surface, set_open, use_popup, Nav, OpenFn};
use crate::theme::ActiveQuiver;

type SelectFn = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct MenuItem {
    label: SharedString,
    icon: Option<AnyElement>,
    destructive: bool,
    disabled: bool,
    on_select: Option<SelectFn>,
    probe_id: Option<SharedString>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            destructive: false,
            disabled: false,
            on_select: None,
            probe_id: None,
        }
    }

    /// Reports this item to the e2e bridge under `id` while the menu is open.
    pub fn probe(mut self, id: impl Into<SharedString>) -> Self {
        self.probe_id = Some(id.into());
        self
    }

    /// `variant="destructive"`.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(icon.into_any_element());
        self
    }

    pub fn on_select(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    pub(crate) fn is_destructive(&self) -> bool {
        self.destructive
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub(crate) fn label(&self) -> &SharedString {
        &self.label
    }

    pub(crate) fn selector(&self) -> Option<SelectFn> {
        self.on_select.clone()
    }
}

pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAlign {
    Start,
    #[default]
    End,
}

#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    trigger: Button,
    entries: Vec<MenuEntry>,
    align: MenuAlign,
    on_open_change: Option<OpenFn>,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>, trigger: Button) -> Self {
        Self {
            id: id.into(),
            trigger,
            entries: Vec::new(),
            align: MenuAlign::default(),
            on_open_change: None,
        }
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Called when the popup opens or closes.
    pub fn on_open_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.quiver().palette;
        let trigger_id = self.trigger.element_id().clone();
        let h = use_popup(&self.id, &trigger_id, window, cx);
        let (open, highlighted) = {
            let s = h.state.read(cx);
            (s.open, s.highlighted)
        };
        let disabled: Vec<bool> = self
            .entries
            .iter()
            .map(|e| matches!(e, MenuEntry::Item(i) if i.disabled))
            .collect();
        let selects: Vec<Option<SelectFn>> = self
            .entries
            .iter()
            .map(|e| {
                if let MenuEntry::Item(i) = e {
                    i.on_select.clone()
                } else {
                    None
                }
            })
            .collect();
        let separators: Vec<bool> = self
            .entries
            .iter()
            .map(|e| matches!(e, MenuEntry::Separator))
            .collect();
        let nav_disabled: Vec<bool> = disabled
            .iter()
            .zip(&separators)
            .map(|(d, s)| *d || *s)
            .collect();

        let (st, pf) = (h.state.clone(), h.popup_focus.clone());
        let oo_trigger = self.on_open_change.clone();
        let trigger = self
            .trigger
            .pressed(open)
            .on_click(move |e: &ClickEvent, window, cx| {
                let keyboard = matches!(e, ClickEvent::Keyboard(_));
                if keyboard && st.update(cx, |s, _| std::mem::take(&mut s.swallow_click)) {
                    return;
                }
                let was_open = {
                    let s = st.read(cx);
                    if keyboard {
                        s.open
                    } else {
                        s.was_open_at_press
                    }
                };
                set_open(&st, !was_open, None, &oo_trigger, window, cx);
                if !was_open {
                    window.focus(&pf);
                }
            });
        let st = h.state.clone();
        let trigger_el = div()
            .capture_any_mouse_down(move |_: &MouseDownEvent, _, cx| {
                st.update(cx, |s, _| s.was_open_at_press = s.open)
            })
            .child(trigger)
            .into_any_element();

        let popup = open.then(|| {
            let (st_key, _pf_key, tf_key) = (
                h.state.clone(),
                h.popup_focus.clone(),
                h.trigger_focus.clone(),
            );
            let (nd, sel) = (nav_disabled.clone(), selects.clone());
            let (oo_out, oo_key) = (self.on_open_change.clone(), self.on_open_change.clone());
            let st_out = h.state.clone();
            let items: Vec<AnyElement> = self
                .entries
                .into_iter()
                .enumerate()
                .map(|(i, entry)| match entry {
                    MenuEntry::Separator => div()
                        .h(px(1.))
                        .bg(p.border)
                        .mx(px(-4.))
                        .my(px(4.))
                        .into_any_element(),
                    MenuEntry::Item(item) => {
                        let lit = highlighted == Some(i);
                        let (fg, hover_bg) = if item.destructive {
                            (p.destructive, p.destructive.opacity(0.08))
                        } else {
                            (p.popover_foreground, p.accent)
                        };
                        let icon_color = if item.destructive {
                            p.destructive
                        } else {
                            p.muted_foreground
                        };
                        let (st_hover, st_click, tf_click) =
                            (h.state.clone(), h.state.clone(), h.trigger_focus.clone());
                        let oo_item = self.on_open_change.clone();
                        let on_select = item.on_select.clone();
                        let key = format!("{}-item-{}", self.id, i);
                        let el = div()
                            .id(child_id(&self.id, format!("item-{i}")))
                            .debug_selector(move || key)
                            .relative()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .rounded(px(Radius::MD))
                            .px(px(8.))
                            .py(px(6.))
                            .text_size(px(14.))
                            .text_color(fg)
                            .when(lit, |d| d.bg(hover_bg))
                            .children(item.icon.map(|ic| div().text_color(icon_color).child(ic)))
                            .when_some(item.probe_id, |d, id| {
                                d.child(probe(
                                    id,
                                    Kind::MenuItem,
                                    item.label.clone(),
                                    !item.disabled,
                                    "",
                                ))
                            })
                            .child(item.label);
                        if item.disabled {
                            return el.opacity(0.64).into_any_element();
                        }
                        el.cursor_pointer()
                            .on_mouse_move(move |_: &MouseMoveEvent, _, cx| {
                                st_hover.update(cx, |s, cx| {
                                    if s.highlighted != Some(i) {
                                        s.highlighted = Some(i);
                                        cx.notify();
                                    }
                                })
                            })
                            .on_click(move |_, window, cx| {
                                set_open(&st_click, false, None, &oo_item, window, cx);
                                window.focus(&tf_click);
                                if let Some(f) = &on_select {
                                    f(window, cx);
                                }
                            })
                            .into_any_element()
                    }
                })
                .collect();
            popup_surface(cx)
                .id(child_id(&self.id, "popup"))
                .min_w(px(160.))
                .track_focus(&h.popup_focus)
                .key_context("QuiverMenu")
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
                            if let Some(Some(f)) = sel.get(i) {
                                f(window, cx);
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
                .children(items)
                .into_any_element()
        });
        floating(trigger_el, popup, self.align == MenuAlign::End)
    }
}
