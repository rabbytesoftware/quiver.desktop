//! Gallery page: tabs, menu, tooltip.

use gpui::prelude::*;
use gpui::{div, AnyElement, Context};

use super::layout::{row, section};
use super::state::Gallery;
use crate::{
    tooltip, Button, ButtonSize, ButtonVariant, Menu, MenuAlign, MenuItem, Orientation, TabItem,
    Tabs, TabsVariant,
};

pub(super) fn page(g: &mut Gallery, cx: &mut Context<Gallery>) -> Vec<AnyElement> {
    let tabs =
        |id: &'static str, variant, orientation, sel: usize, set: fn(&mut Gallery, usize)| {
            Tabs::new(id)
                .variant(variant)
                .orientation(orientation)
                .selected(sel)
                .tab(TabItem::new("General"))
                .tab(TabItem::new("Engine"))
                .tab(TabItem::new("Developer"))
                .tab(TabItem::new("Disabled").disabled(true))
                .on_change(cx.processor(move |this: &mut Gallery, i, _, cx| {
                    set(this, i);
                    cx.notify();
                }))
                .panel(div().p(gpui::px(8.)).child(format!("Panel {sel}")))
                .into_any_element()
        };
    let tab_rows = vec![
        row(
            "default",
            vec![tabs(
                "g-tabs",
                TabsVariant::Default,
                Orientation::Horizontal,
                g.tab,
                |g, i| g.tab = i,
            )],
            cx,
        ),
        row(
            "underline",
            vec![tabs(
                "g-tabs-u",
                TabsVariant::Underline,
                Orientation::Horizontal,
                g.underline_tab,
                |g, i| g.underline_tab = i,
            )],
            cx,
        ),
        row(
            "vertical",
            vec![tabs(
                "g-tabs-v",
                TabsVariant::Default,
                Orientation::Vertical,
                g.tab,
                |g, i| g.tab = i,
            )],
            cx,
        ),
    ];
    let me = cx.entity();
    let note = move |label: &'static str| {
        let me = me.clone();
        move |_: &mut gpui::Window, cx: &mut gpui::App| {
            me.update(cx, |this, cx| {
                this.last_action = label.into();
                cx.notify();
            })
        }
    };
    let menu = Menu::new(
        "g-menu",
        Button::new("g-menu-trigger")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .label("Actions"),
    )
    .align(MenuAlign::Start)
    .item(MenuItem::new("Rename").on_select(note("Rename")))
    .item(MenuItem::new("Duplicate").on_select(note("Duplicate")))
    .item(MenuItem::new("Archive").disabled(true))
    .separator()
    .item(
        MenuItem::new("Delete")
            .destructive(true)
            .on_select(note("Delete")),
    );
    let menu_rows = vec![row(
        "popup",
        vec![
            menu.into_any_element(),
            div()
                .child(format!("last: {}", g.last_action))
                .into_any_element(),
        ],
        cx,
    )];
    let tip_rows = vec![row(
        "hover (~500ms)",
        vec![
            div()
                .id("g-tip-a")
                .px(gpui::px(10.))
                .py(gpui::px(6.))
                .child("Hover me")
                .tooltip(tooltip("Settings"))
                .into_any_element(),
            Button::new("g-tip-b")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .label("?")
                .tooltip(tooltip("Help"))
                .into_any_element(),
        ],
        cx,
    )];
    vec![
        section("Tabs", tab_rows, cx),
        section("Menu", menu_rows, cx),
        section("Tooltip", tip_rows, cx),
    ]
}
