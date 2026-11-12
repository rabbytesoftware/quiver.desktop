//! The sidebar's footer: the build indicator (core and app lines). Click opens the console.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::console::build_info::{
    compact_for_width, describe_core, describe_desktop, format_build, DesktopStamp, FormatOptions,
};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, TrackedText};

use super::super::Sidebar;

/// Nothing at all when the user has the indicator turned off.
pub fn build_footer(
    sidebar: &Sidebar,
    cx: &mut Context<Sidebar>,
) -> Option<impl IntoElement> {
    let p = cx.quiver().palette;
    let app = sidebar.app.read(cx);
    if !crate::model::build_indicator(app).shown() {
        return None;
    }
    let options = FormatOptions {
        compact: compact_for_width(app.shell.sidebar_width),
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
    let mono = FontFamily::Mono.family();
    let line = |label: &'static str, value: String| {
        div()
            .flex()
            .min_w_0()
            .gap(px(8.))
            .whitespace_nowrap()
            .child(div().w(px(26.)).flex_none().opacity(0.7).child(label))
            .child(TrackedText::inherit_color(value, 10., 12., 0.).family(mono))
    };
    let console_app = sidebar.app.clone();
    Some(
        div()
            .flex_none()
            .px(px(6.))
            .py(px(4.))
            .border_t_1()
            .border_color(p.sidebar_border)
            .child(
                div()
                    .id("sidebar-build")
                    .relative()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .h(px(30.))
                    .max_w_full()
                    .min_w_0()
                    .px(px(6.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_color(p.muted_foreground)
                    .hover(|s| s.bg(p.sidebar_element_hover).text_color(p.foreground))
                    .on_click(move |_, _, cx| {
                        console_app.update(cx, |m, cx| {
                            let open = !m.console_open;
                            m.set_console_open(open, cx);
                        })
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
            ),
    )
}
