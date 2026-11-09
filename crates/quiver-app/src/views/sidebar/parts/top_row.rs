//! The 34px top row: build indicator (core and app lines), connection name, back and forward.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::console::build_info::{
    compact_for_width, describe_core, describe_desktop, format_build, DesktopStamp, FormatOptions,
};
use quiver_state::shell::rail_owns_controls;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Glyph};

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
        p.foreground_subtle.opacity(0.6)
    };
    div()
        .id(id)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded(px(6.))
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(p.accent))
                .on_click(move |_, _, cx| app.update(cx, |m, cx| on_click(m, cx)))
        })
        .child(icon(glyph, 16., color))
        .child(probe(probe_id, Kind::Button, id, enabled, ""))
}

pub fn top_row(sidebar: &Sidebar, cx: &mut Context<Sidebar>) -> impl IntoElement {
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
    let (can_back, can_forward) = (app.history.can_go_back(), app.history.can_go_forward());
    let reserve = rail_owns_controls(cfg!(target_os = "macos"), app.shell.sidebar_side);
    let mono = FontFamily::Mono.family();
    let line = |label: &'static str, value: String| {
        div()
            .flex()
            .gap(px(5.))
            .child(div().text_color(p.foreground_subtle).child(label))
            .child(div().text_color(p.sidebar_foreground).child(value))
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(4.))
        .h(px(34.))
        .px(px(6.))
        .when(reserve, |d| d.pl(px(64.)))
        .child(
            div()
                .relative()
                .pl(px(4.))
                .font_family(mono)
                .text_size(px(8.5))
                .line_height(px(10.))
                .when_some(core_line, |d, core| d.child(line("core", core)))
                .child(line("app", app_line))
                .child(probe("sidebar.build", Kind::Text, "Build", true, "")),
        )
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(5.))
                .ml(px(4.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .child(div().size(px(6.)).rounded_full().bg(p.primary))
                .child("Local")
                .child(probe("sidebar.connection", Kind::Text, "Local", true, "")),
        )
        .child(div().flex_1())
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
