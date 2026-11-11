//! The 34px top row: build indicator (core and app lines), connection name, back and forward.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::console::build_info::{
    compact_for_width, describe_core, describe_desktop, format_build, DesktopStamp, FormatOptions,
};
use quiver_state::domain::ConnectionStatus;
use quiver_state::shell::rail_owns_controls;
use quiver_theme::FontFamily;
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
    let compact = compact_for_width(app.shell.sidebar_width);
    let options = FormatOptions {
        compact,
        ..FormatOptions::default()
    };
    let info = app.services.release.build_info();
    let desktop = describe_desktop(&DesktopStamp {
        channel: info.channel.clone(),
        label: info.stamp.label.clone(),
        commit: info.stamp.commit.clone(),
        built_at: info.stamp.built_at.and_then(|s| i64::try_from(s).ok()),
    });
    let app_line = format_build(&desktop, options);
    let core_line = app
        .core_versions
        .read(|v| describe_core(v.as_ref()))
        .map(|core| format_build(&core, options));
    // What the indicator says, for the e2e driver (`sidebar.build` state): the core line appears once /versions answered.
    let build_text = match &core_line {
        Some(core) => format!("core {core}; app {app_line}"),
        None => format!("app {app_line}"),
    };
    let shown = crate::model::build_indicator(app).shown();
    let (can_back, can_forward) = (app.history.can_go_back(), app.history.can_go_forward());
    let reserve = rail_owns_controls(cfg!(target_os = "macos"), app.shell.sidebar_side);
    let mono = FontFamily::Mono.family();
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
    let line = |label: &'static str, value: String| {
        div()
            .flex()
            .min_w_0()
            .gap(px(8.))
            .whitespace_nowrap()
            .child(div().w(px(26.)).flex_none().opacity(0.7).child(label))
            .child(TrackedText::inherit_color(value, 10., 12., 0.).family(mono))
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(34.))
        .px(px(6.))
        .when(reserve, |d| d.pl(px(64.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .when(shown, |d| {
                    d.child(
                        div()
                            .id("sidebar-build")
                            .relative()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .h(px(28.))
                            .max_w_full()
                            .min_w_0()
                            .px(px(6.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(p.muted_foreground)
                            .hover(|s| s.bg(p.sidebar_element_hover).text_color(p.foreground))
                            .on_click({
                                let app = sidebar.app.clone();
                                move |_, _, cx| {
                                    app.update(cx, |m, cx| {
                                        let open = !m.console_open;
                                        m.set_console_open(open, cx);
                                    })
                                }
                            })
                            .font_family(mono)
                            .text_size(px(10.))
                            .line_height(px(12.))
                            .when_some(core_line, |d, core| d.child(line("core", core)))
                            .child(line("app", app_line))
                            .child(probe(
                                "sidebar.build",
                                Kind::Button,
                                "Build",
                                true,
                                build_text,
                            )),
                    )
                }),
        )
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
