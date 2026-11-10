//! The search field and the Home / Remote / Settings tab strip. The active tab shows its name, the
//! others only their icon, with a tooltip.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::settings::SettingsTab;
use quiver_state::sidebar::{active_destination, Destination, DESTINATIONS};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, inner_highlight, shadow_xs, tooltip, ActiveQuiver, Glyph, SearchInput};

use super::super::Sidebar;
use crate::model::Route;

fn glyph(destination: Destination) -> Glyph {
    match destination {
        Destination::Home => Glyph::Home,
        Destination::Remote => Glyph::Server,
        Destination::Settings => Glyph::Settings,
    }
}

fn route(destination: Destination) -> Route {
    match destination {
        Destination::Home => Route::Home,
        Destination::Remote => Route::Remote,
        Destination::Settings => Route::Settings {
            tab: SettingsTab::General,
        },
    }
}

fn tab_id(destination: Destination) -> &'static str {
    match destination {
        Destination::Home => "sidebar.tab.home",
        Destination::Remote => "sidebar.tab.remote",
        Destination::Settings => "sidebar.tab.settings",
    }
}

pub fn search_and_tabs(
    sidebar: &Sidebar,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let app = sidebar.app.clone();
    let active = active_destination(&app.read(cx).route().path());
    let tabs = DESTINATIONS.map(|destination| {
        let is_active = active == Some(destination);
        let label = app.read(cx).t(destination.label());
        let color = if is_active {
            p.primary_foreground
        } else {
            p.foreground.opacity(0.7)
        };
        let app = app.clone();
        let mut tab = div()
            .id(tab_id(destination))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .h(px(32.))
            .rounded(px(6.))
            .flex_basis(px(0.))
            .flex_grow()
            .flex_shrink()
            .cursor_pointer()
            .text_color(color)
            .on_click(move |_, _, cx| {
                app.update(cx, |m, cx| m.navigate(route(destination), cx));
            })
            .child(icon(glyph(destination), 14., color))
            .child(probe(
                tab_id(destination),
                Kind::Tab,
                label.clone(),
                true,
                if is_active { "selected" } else { "" },
            ));
        if is_active {
            tab = tab
                .flex_none()
                .px(px(12.))
                .bg(p.primary)
                .shadow(shadow_xs(gpui::black().opacity(0.10)))
                .child(inner_highlight(6., p.selected_edge))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(label),
                );
        } else {
            tab = tab
                .tooltip(tooltip(label))
                .hover(|s| s.text_color(p.muted_foreground));
        }
        tab
    });
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(6.))
        .px(px(8.))
        .pt(px(2.))
        .pb(px(4.))
        .child(
            SearchInput::new("sidebar-search", &sidebar.search)
                .probes("sidebar.search", "sidebar.search.clear"),
        )
        .child(
            div()
                .flex()
                .gap(px(2.))
                .p(px(2.))
                .rounded(px(8.))
                .bg(p.sidebar_element_idle)
                .children(tabs),
        )
}
