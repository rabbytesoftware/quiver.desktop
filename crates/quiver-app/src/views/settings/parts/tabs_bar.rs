//! The segmented tab control at the top of the page (`settings.tab.<name>`).

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::settings::{visible_tabs, SettingsTab};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::settings::SettingsView;

pub fn tab_name(tab: SettingsTab) -> &'static str {
    match tab {
        SettingsTab::General => "general",
        SettingsTab::Engine => "engine",
        SettingsTab::Developer => "developer",
    }
}

pub fn tabs_bar(
    view: &SettingsView,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let tabs = visible_tabs().into_iter().map(|item| {
        let selected = item.id == view.tab;
        let label = view.app.read(cx).t(item.label);
        let this = cx.entity();
        let tab = item.id;
        div()
            .id(gpui::SharedString::from(format!(
                "settings-tab-{}",
                tab_name(tab)
            )))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .h(px(32.))
            .px(px(10.))
            .rounded(px(8.))
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .when(selected, |d| {
                d.bg(p.primary).text_color(p.primary_foreground)
            })
            .when(!selected, |d| d.text_color(p.foreground_subtle))
            .on_click(move |_, _, cx| this.update(cx, |v, cx| v.set_tab(tab, cx)))
            .child(label.clone())
            .child(probe(
                format!("settings.tab.{}", tab_name(tab)),
                Kind::Tab,
                label,
                true,
                if selected { "selected" } else { "" },
            ))
    });
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(45.))
        .pl(px(12.))
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .p(px(2.))
                .h(px(36.))
                .rounded(px(10.))
                .bg(p.muted)
                .children(tabs),
        )
}
