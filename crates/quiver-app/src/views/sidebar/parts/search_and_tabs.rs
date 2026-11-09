//! The search field and the Home / Remote / Settings tab strip. The active tab shows its name, the
//! others only their icon, with a tooltip.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::settings::SettingsTab;
use quiver_state::sidebar::{active_destination, Destination, DESTINATIONS};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, tooltip, ActiveQuiver, Glyph, SearchInput};

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

pub fn search_and_tabs(sidebar: &Sidebar, cx: &mut Context<Sidebar>) -> impl IntoElement {
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
            .gap(px(7.))
            .rounded(px(8.))
            .flex_basis(px(0.))
            .flex_grow()
            .flex_shrink()
            .cursor_pointer()
            .text_color(color)
            .on_click(move |_, _, cx| {
                app.update(cx, |m, cx| m.navigate(route(destination), cx));
            })
            .child(icon(glyph(destination), 16., color))
            .child(probe(
                tab_id(destination),
                Kind::Tab,
                label.clone(),
                true,
                if is_active { "selected" } else { "" },
            ));
        if is_active {
            tab = tab.bg(p.primary).shadow(quiver_ui_shadow()).child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            );
            tab.style().flex_grow = Some(2.4);
        } else {
            tab = tab
                .tooltip(tooltip(label))
                .hover(|s| s.bg(p.background.opacity(0.5)));
            tab.style().flex_grow = Some(1.0);
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
                .h(px(36.))
                .rounded(px(8.))
                .bg(p.border)
                .children(tabs),
        )
}

fn quiver_ui_shadow() -> Vec<gpui::BoxShadow> {
    vec![gpui::BoxShadow {
        color: gpui::black().opacity(0.15),
        offset: gpui::point(px(0.), px(1.)),
        blur_radius: px(2.),
        spread_radius: px(0.),
    }]
}
