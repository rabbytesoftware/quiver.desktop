//! One arrow row: gradient icon, name (and namespace@version when selected), status glyph.

use gpui::prelude::*;
use gpui::{
    div, hsla, linear_color_stop, linear_gradient, px, white, FontWeight, IntoElement, MouseButton,
    SharedString,
};
use quiver_state::arrow_details::{RunState, StatusGlyph as StateGlyph};
use quiver_state::domain::ArrowEntry;
use quiver_state::namespace::bare_namespace;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};

use quiver_ui::{
    icon_with_stroke, inner_highlight, shadow_xs, ActiveQuiver, Glyph, GlyphKind, StatusGlyph,
};

use super::super::Sidebar;
use crate::model::Route;

fn glyph_kind(glyph: StateGlyph) -> Option<GlyphKind> {
    match glyph {
        StateGlyph::Busy => Some(GlyphKind::Busy),
        StateGlyph::Running => Some(GlyphKind::Running),
        StateGlyph::NeedsAttention => Some(GlyphKind::NeedsAttention),
        StateGlyph::UpdateAvailable => Some(GlyphKind::UpdateAvailable),
        StateGlyph::Ready | StateGlyph::Inactive => None,
    }
}

/// The e2e name of a glyph (`sidebar.row.<ns>.glyph` label).
fn glyph_name(kind: GlyphKind) -> &'static str {
    match kind {
        GlyphKind::Busy => "busy",
        GlyphKind::Running => "running",
        GlyphKind::NeedsAttention => "needs-attention",
        GlyphKind::UpdateAvailable => "update-available",
    }
}

fn mock_palette(namespace: &str) -> (f32, f32) {
    let mut hash: u32 = 5381;
    for unit in namespace.encode_utf16() {
        hash = hash.wrapping_mul(33) ^ u32::from(unit);
    }
    hash = (hash ^ (hash >> 16)).wrapping_mul(0x7feb_352d);
    hash = (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;
    let hue = f32::from((hash % 24) as u8) * 15.0 / 360.0;
    let lift = f32::from(((hash >> 8) % 2) as u8) * 0.09;
    (hue, lift)
}

pub fn arrow_row(
    sidebar: &mut Sidebar,
    entry: ArrowEntry,
    active: bool,
    cx: &mut gpui::Context<Sidebar>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let namespace = bare_namespace(&entry.namespace).to_owned();
    let (hue, lift) = mock_palette(&entry.namespace);
    let glyph = glyph_kind(StateGlyph::for_state(
        RunState::from(entry.state),
        true,
        false,
    ));
    let color = match glyph {
        Some(GlyphKind::NeedsAttention) if active => p.destructive.opacity(0.7),
        Some(GlyphKind::NeedsAttention) => p.destructive,
        Some(GlyphKind::Busy) if !active => p.muted_foreground,
        _ if active => p.primary_foreground,
        _ => p.primary,
    };
    let (fg, mono) = (
        if active {
            p.primary_foreground
        } else {
            p.foreground
        },
        FontFamily::Mono.family(),
    );
    let (app, ns_click, ns_menu) = (sidebar.app.clone(), namespace.clone(), namespace.clone());
    let menu_state = cx.entity();
    let probe_id = format!("sidebar.row.{namespace}");
    div().h(px(40.)).py(px(2.)).child(
        div()
            .id(SharedString::from(format!("sidebar-row-{namespace}")))
            .relative()
            .flex()
            .items_center()
            .gap(px(6.))
            .h(px(36.))
            .px(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(if active {
                p.primary
            } else {
                gpui::transparent_black()
            })
            .text_color(fg)
            .cursor_pointer()
            .when(active, |d| {
                d.bg(p.primary)
                    .shadow(shadow_xs(gpui::black().opacity(0.10)))
                    .child(inner_highlight(8., p.selected_edge))
            })
            .when(!active, |d| d.hover(|s| s.bg(p.accent)))
            .on_click(move |_, _, cx| {
                let namespace = ns_click.clone();
                let route = Route::Arrow { namespace };
                app.update(cx, |m, cx| m.navigate(route, cx));
            })
            .on_mouse_down(MouseButton::Right, move |e, _, cx| {
                let target = (ns_menu.clone(), e.position);
                menu_state.update(cx, |s, cx| {
                    s.context = Some(target);
                    cx.notify();
                });
            })
            .child(
                div()
                    .flex_none()
                    .size(px(20.))
                    .rounded(px(4.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(hsla(hue, 0.52, 0.38 + lift, 1.), 0.5),
                        linear_color_stop(hsla(hue, 0.48, 0.30 + lift, 1.), 0.5),
                    ))
                    .child(icon_with_stroke(
                        Glyph::ArrowIcon,
                        20.,
                        white().opacity(0.92),
                        1.6,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(entry.name.clone()),
                    )
                    .when(active, |d| {
                        d.child(
                            div()
                                .flex()
                                .min_w_0()
                                .font_family(mono)
                                .text_size(px(10.5))
                                .line_height(px(13.))
                                .text_color(p.primary_foreground.opacity(0.6))
                                .child(div().min_w_0().truncate().child(namespace.clone()))
                                .child(div().flex_none().child(format!("@{}", entry.version))),
                        )
                    }),
            )
            .children(glyph.map(|kind| {
                StatusGlyph::new(
                    SharedString::from(format!("sidebar-glyph-{namespace}")),
                    kind,
                )
                .size(16.)
                .color(color)
            }))
            .children(glyph.map(|kind| {
                probe(
                    format!("sidebar.row.{namespace}.glyph"),
                    Kind::Other,
                    glyph_name(kind),
                    true,
                    "",
                )
            }))
            .child(probe(
                probe_id,
                Kind::Button,
                entry.name,
                true,
                if active { "selected" } else { "" },
            )),
    )
}
