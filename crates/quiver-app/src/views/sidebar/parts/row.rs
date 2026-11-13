//! One arrow row: gradient icon, name (and namespace@version when selected), status glyph.

use gpui::prelude::*;
use gpui::{div, px, FontWeight, IntoElement, MouseButton, SharedString};
use quiver_state::arrow_details::{RunState, StatusGlyph as StateGlyph};
use quiver_state::domain::ArrowEntry;
use quiver_state::namespace::bare_namespace;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};

use quiver_ui::{inner_highlight, shadow_xs, ActiveQuiver, GlyphKind, StatusGlyph, TrackedText};

use super::super::Sidebar;
use crate::model::Route;
use crate::services::native_menu::MenuAnchor;

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

pub fn arrow_row(
    sidebar: &mut Sidebar,
    entry: ArrowEntry,
    active: bool,
    cx: &mut gpui::Context<Sidebar>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let namespace = bare_namespace(&entry.namespace).to_owned();
    let icon_url: Option<gpui::SharedString> =
        entry.icon.clone().filter(|u| !u.is_empty()).map(Into::into);
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
            // The OS's own menu. macOS opens it on press; elsewhere on release, which also leaves the
            // pointer ungrabbed for GTK's popup on X11.
            .on_mouse_down(MouseButton::Right, {
                let (ns, state) = (ns_menu.clone(), menu_state.clone());
                move |e, window, cx| {
                    if cfg!(target_os = "macos") {
                        let anchor = MenuAnchor::of(window);
                        let (ns, at) = (ns.clone(), e.position);
                        state.update(cx, |s, cx| s.open_row_menu(ns, at, anchor, cx));
                    }
                }
            })
            .on_mouse_up(MouseButton::Right, move |e, window, cx| {
                if !cfg!(target_os = "macos") {
                    let anchor = MenuAnchor::of(window);
                    let (ns, at) = (ns_menu.clone(), e.position);
                    menu_state.update(cx, |s, cx| s.open_row_menu(ns, at, anchor, cx));
                }
            })
            .child({
                let initial: gpui::SharedString = entry
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>())
                    .unwrap_or_else(|| "?".into())
                    .into();
                let chip = quiver_ui::monogram(
                    quiver_state::tile::hue_for(&entry.namespace),
                    20.,
                    4.,
                    &initial,
                );
                // Old `ArrowIcon`: the arrow's own icon replaces the drawn chip once it has loaded; until then, or
                // if it cannot, the chip shows.
                div()
                    .relative()
                    .flex_none()
                    .size(px(20.))
                    .rounded(px(4.))
                    .child(match icon_url.clone() {
                        Some(url) => quiver_ui::Photo::new(url)
                            .radius(4.)
                            .stand_in(chip)
                            .into_any_element(),
                        None => chip.into_any_element(),
                    })
            })
            .child(
                div()
                    .relative()
                    .top(px(2.))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(
                        TrackedText::new(entry.name.clone(), 13., 16., -0.1, fg)
                            .weight(FontWeight::MEDIUM),
                    )
                    .when(active, |d| {
                        let meta = p.primary_foreground.opacity(0.6);
                        d.child(
                            div()
                                .flex()
                                .min_w_0()
                                .child(
                                    TrackedText::new(namespace.clone(), 10.5, 13., -0.2, meta)
                                        .family(mono),
                                )
                                .child(
                                    div().flex_none().child(
                                        TrackedText::new(
                                            followed(&entry.namespace, &entry.version),
                                            10.5,
                                            13.,
                                            -0.2,
                                            meta,
                                        )
                                        .family(mono),
                                    ),
                                ),
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

/// What the row follows, as the arrow page says it (old `arrow-row.tsx`): `@selector`, then `· version` when the
/// selector resolved to something else (`@stable · v27.7.15`); `@version` when there is no selector.
fn followed(
    namespace: &str,
    version: &str,
) -> String {
    match namespace.split_once('@').map(|(_, s)| s) {
        Some(selector) if !selector.is_empty() => {
            if version.is_empty() || version == selector {
                format!("@{selector}")
            } else {
                format!("@{selector} · {version}")
            }
        }
        _ => format!("@{version}"),
    }
}

#[cfg(test)]
mod tests {
    use super::followed;

    #[test]
    fn the_row_names_what_it_follows_then_what_that_resolved_to() {
        assert_eq!(
            followed("github.com/a/b@stable", "v27.7.15"),
            "@stable · v27.7.15"
        );
        assert_eq!(followed("github.com/a/b@v1.2", "v1.2"), "@v1.2");
        assert_eq!(followed("github.com/a/b@stable", ""), "@stable");
        assert_eq!(followed("github.com/a/b", "v3"), "@v3");
    }
}
