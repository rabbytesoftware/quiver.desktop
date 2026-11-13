//! `ContextMenu`: the Menu popup opened at a window position (right click on a row) instead of
//! under a trigger. Rendered by the owner while it holds an open position; Escape, an outside press
//! and choosing an item all call `on_dismiss`.

use std::rc::Rc;

use crate::layer::layer;
use gpui::prelude::*;
use gpui::{anchored, div, px, App, Corner, ElementId, KeyDownEvent, Pixels, Point, Window};
use quiver_theme::Radius;

use crate::ids::child_id;
use crate::inspect::{probe, Kind};
use crate::menu::{MenuEntry, MenuItem};
use crate::popup::popup_surface;
use crate::theme::ActiveQuiver;

type DismissFn = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    position: Point<Pixels>,
    entries: Vec<MenuEntry>,
    on_dismiss: Option<DismissFn>,
    probe_prefix: Option<gpui::SharedString>,
}

impl ContextMenu {
    pub fn new(
        id: impl Into<ElementId>,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            id: id.into(),
            position,
            entries: Vec::new(),
            on_dismiss: None,
            probe_prefix: None,
        }
    }

    /// E2e ids of the items: `<prefix>.<label, lower-cased with dashes>`.
    pub fn probes(
        mut self,
        prefix: impl Into<gpui::SharedString>,
    ) -> Self {
        self.probe_prefix = Some(prefix.into());
        self
    }

    pub fn item(
        mut self,
        item: MenuItem,
    ) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    pub fn on_dismiss(
        mut self,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ContextMenu {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let dismiss = self.on_dismiss;
        let prefix = self.probe_prefix;
        let items: Vec<gpui::AnyElement> = self
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
                    let (fg, hover_bg) = if item.is_destructive() {
                        (p.destructive, p.destructive.opacity(0.08))
                    } else {
                        (p.popover_foreground, p.accent)
                    };
                    let disabled = item.is_disabled();
                    let (label, on_select) = (item.label().clone(), item.selector());
                    let dismiss = dismiss.clone();
                    let probe_id = prefix
                        .as_ref()
                        .map(|p| format!("{p}.{}", label.to_lowercase().replace(' ', "-")));
                    let el = div()
                        .id(child_id(&self.id, format!("item-{i}")))
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .rounded(px(Radius::MD))
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(14.))
                        .text_color(fg)
                        .child(label.clone())
                        .when_some(probe_id, |d, id| {
                            d.child(probe(id, Kind::MenuItem, label, !disabled, ""))
                        });
                    if disabled {
                        return el.opacity(0.64).into_any_element();
                    }
                    el.cursor_pointer()
                        .hover(move |s| s.bg(hover_bg))
                        .on_click(move |_, window, cx| {
                            if let Some(f) = &dismiss {
                                f(window, cx);
                            }
                            if let Some(f) = &on_select {
                                f(window, cx);
                            }
                        })
                        .into_any_element()
                }
            })
            .collect();
        let on_out = dismiss.clone();
        let on_key = dismiss;
        layer(
            anchored()
                .position(self.position)
                .anchor(Corner::TopLeft)
                .snap_to_window_with_margin(px(8.))
                .child(
                    popup_surface(cx)
                        .id(child_id(&self.id, "popup"))
                        .min_w(px(180.))
                        .on_mouse_down_out(move |_, window, cx| {
                            if let Some(f) = &on_out {
                                f(window, cx);
                            }
                        })
                        .on_key_down(move |e: &KeyDownEvent, window, cx| {
                            if e.keystroke.key == "escape" {
                                if let Some(f) = &on_key {
                                    f(window, cx);
                                }
                                cx.stop_propagation();
                            }
                        })
                        .children(items),
                ),
        )
        .with_priority(2)
    }
}
