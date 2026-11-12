//! The 34px top row: connection name, back and forward (the build indicator is the footer).

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::domain::ConnectionStatus;
use quiver_state::shell::rail_owns_controls;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Glyph, TrackedText};

use super::super::Sidebar;
use crate::model::AppModel;

fn nav_button(
    id: &'static str,
    probe_id: &'static str,
    glyph: Glyph,
    enabled: bool,
    app: gpui::Entity<AppModel>,
    cx: &Context<Sidebar>,
    on_click: impl Fn(&mut AppModel, &mut Context<AppModel>) + 'static,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let color = if enabled {
        p.muted_foreground
    } else {
        p.muted_foreground.opacity(0.3)
    };
    div()
        .id(id)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.))
        .m(px(2.))
        .rounded(px(4.))
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(p.sidebar_element_hover))
                .on_click(move |_, _, cx| app.update(cx, |m, cx| on_click(m, cx)))
        })
        .child(icon(glyph, 16., color))
        .child(probe(probe_id, Kind::Button, id, enabled, ""))
}

pub fn top_row(
    sidebar: &Sidebar,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let app = sidebar.app.read(cx);
    let (can_back, can_forward) = (app.history.can_go_back(), app.history.can_go_forward());
    let reserve = rail_owns_controls(cfg!(target_os = "macos"), app.shell.sidebar_side);
    let connections = app.services.connections.snapshot();
    let status = app.services.status.status();
    let switcher = (connections.connections.len() > 1).then(|| {
        let name = connections
            .connections
            .iter()
            .find(|c| c.id == connections.active_id)
            .map_or_else(
                || app.t(quiver_state::i18n::en::nav::REMOTE),
                |c| c.name.clone(),
            );
        let dot = div().flex_none().size(px(6.)).rounded_full();
        let dot = match status {
            ConnectionStatus::Ready => dot.bg(p.foreground),
            ConnectionStatus::Starting => dot.bg(p.foreground_subtle),
            ConnectionStatus::Disconnected => dot.border_1().border_color(p.destructive),
        };
        div()
            .id("sidebar-connection")
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .mr(px(4.))
            .max_w(px(98.))
            .h(px(24.))
            .px(px(6.))
            .rounded_full()
            .cursor_pointer()
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .hover(|s| s.bg(p.sidebar_element_hover))
            // The old `ConnectionSwitcher`: the chip opens the command palette.
            .on_click(|_, _, cx| crate::views::remote::open_palette(cx))
            .child(dot)
            // Painted by `TrackedText` so a long name ends in "…": gpui only ellipsizes text whose box
            // has a definite width, and this one is flex-shrunk.
            .child(
                TrackedText::inherit_color(name.clone(), 11.5, 14., 0.).weight(FontWeight::MEDIUM),
            )
            .child(probe("sidebar.connection", Kind::Text, name, true, ""))
    });
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(34.))
        .px(px(6.))
        .when(reserve, |d| d.pl(px(64.)))
        .child(div().flex_1().min_w_0())
        .children(switcher)
        .child(nav_button(
            "sidebar-back",
            "sidebar.back",
            Glyph::ArrowLeft,
            can_back,
            sidebar.app.clone(),
            cx,
            |m, cx| m.back(cx),
        ))
        .child(nav_button(
            "sidebar-forward",
            "sidebar.forward",
            Glyph::ArrowRight,
            can_forward,
            sidebar.app.clone(),
            cx,
            |m, cx| m.forward(cx),
        ))
}
