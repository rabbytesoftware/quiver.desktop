//! The 44px header: icon, name, status, version, then Reload, Details, the actions menu and Stop.

use gpui::prelude::*;
use gpui::{div, hsla, linear_color_stop, linear_gradient, px, Context, FontWeight, IntoElement};
use quiver_state::arrow_app::{build_pane_header, PaneMenuItem};
use quiver_state::i18n::en::arrowapp;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon, ActiveQuiver, Button, ButtonSize, ButtonVariant, Glyph, Menu, MenuItem, Spinner,
};

use crate::bridge::spawn_detached;
use crate::model::Route;
use crate::views::arrow_app::ArrowAppView;

fn hue(namespace: &str) -> f32 {
    let sum: u32 = namespace.bytes().map(u32::from).sum();
    (sum * 37 % 360) as f32 / 360.0
}

fn menu(
    view: &ArrowAppView,
    items: &[PaneMenuItem],
    cx: &Context<ArrowAppView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let trigger = Button::new("arrow-app-menu")
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::IconSm)
        .child(icon(Glyph::Ellipsis, 16., p.muted_foreground));
    let mut menu = Menu::new("arrow-app-menu-popup", trigger);
    for item in items {
        let label = view.app.read(cx).t(item.label());
        let (app, namespace) = (view.app.clone(), view.namespace.clone());
        // Settings, Methods and Uninstall ask for input: they live on the arrow's own page.
        menu = menu.item(
            MenuItem::new(label)
                .destructive(item.destructive())
                .on_select(move |_, cx| {
                    let route = Route::Arrow {
                        namespace: namespace.clone(),
                    };
                    app.update(cx, |m, cx| m.navigate(route, cx));
                }),
        );
    }
    div()
        .relative()
        .child(menu)
        .child(probe("arrow-app.menu", Kind::Button, "menu", true, ""))
}

fn action(
    id: &'static str,
    probe_id: &'static str,
    label: String,
    variant: ButtonVariant,
    glyph: Option<Glyph>,
    cx: &Context<ArrowAppView>,
    on_click: impl Fn(&mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .child(
            Button::new(id)
                .variant(variant)
                .size(ButtonSize::Sm)
                .when_some(glyph, |b, g| b.child(icon(g, 13., p.foreground)))
                .label(label.clone())
                .on_click(move |_, window, cx| on_click(window, cx)),
        )
        .child(probe(probe_id, Kind::Button, label, true, ""))
}

pub fn header(view: &mut ArrowAppView, cx: &mut Context<ArrowAppView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let entry = view.app.read(cx).services.arrows.entry(&view.namespace);
    let pane = build_pane_header(&view.namespace, entry.as_ref(), true, true);
    let (running, starting) = {
        let m = view.app.read(cx);
        (m.t(arrowapp::RUNNING), m.t(arrowapp::STARTING))
    };
    let (reload, details, stop) = {
        let m = view.app.read(cx);
        (
            m.t(arrowapp::RELOAD),
            m.t(arrowapp::DETAILS),
            m.t(arrowapp::STOP),
        )
    };
    let h = hue(&view.namespace);
    let tint = |s, l| hsla(h, s, l, 1.);
    let status = if pane.ready { running } else { starting };
    let version = entry
        .as_ref()
        .map(|a| a.version.clone())
        .unwrap_or_default();

    let (app, ns) = (view.app.clone(), view.namespace.clone());
    let reload_btn = {
        let (app, ns) = (app.clone(), ns.clone());
        action(
            "arrow-app-reload",
            "arrow-app.reload",
            reload,
            ButtonVariant::Ghost,
            None,
            cx,
            move |_, cx| {
                app.read(cx).services.arrow_windows.reload(&ns);
            },
        )
    };
    let details_btn = {
        let (app, ns) = (app.clone(), ns.clone());
        action(
            "arrow-app-details",
            "arrow-app.details",
            details,
            ButtonVariant::Outline,
            None,
            cx,
            move |_, cx| {
                let route = Route::Arrow {
                    namespace: ns.clone(),
                };
                app.update(cx, |m, cx| m.navigate(route, cx));
            },
        )
    };
    let stop_btn = {
        let (app, ns) = (app.clone(), ns.clone());
        action(
            "arrow-app-stop",
            "arrow-app.stop",
            stop,
            ButtonVariant::DestructiveOutline,
            Some(Glyph::Close),
            cx,
            move |_, cx| {
                let m = app.read(cx);
                m.services.arrow_windows.stop(&ns);
                if m.demo {
                    return;
                }
                let mutations = Mutations::new(m.services.api.clone(), m.services.queries.clone());
                let ns = ns.clone();
                spawn_detached(m.handle(), async move {
                    if let Err(err) = mutations.stop(&ns, &Variables::new()).await {
                        log::warn!("stopping {ns} failed: {err}");
                    }
                });
            },
        )
    };
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(12.))
        .h(px(44.))
        .px(px(16.))
        .border_b_1()
        .border_color(p.border)
        .child(
            div()
                .flex_none()
                .size(px(24.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(tint(0.48, 0.50), 0.),
                    linear_color_stop(tint(0.52, 0.33), 1.),
                ))
                .child(icon(Glyph::Mark, 15., gpui::white())),
        )
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .truncate()
                .child(pane.name.clone()),
        )
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
                .h(px(18.))
                .px(px(6.))
                .rounded(px(5.))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .when(pane.ready, |d| {
                    d.bg(p.primary).text_color(p.primary_foreground)
                })
                .when(!pane.ready, |d| {
                    d.bg(p.muted).text_color(p.muted_foreground)
                })
                .when(!pane.ready, |d| {
                    d.child(
                        Spinner::new("arrow-app-starting")
                            .size(10.)
                            .color(p.muted_foreground),
                    )
                })
                .child(status.clone())
                .child(probe(
                    "arrow-app.status",
                    Kind::Text,
                    status,
                    true,
                    if pane.ready { "ready" } else { "starting" },
                )),
        )
        .child(
            div()
                .font_family(FontFamily::Mono.family())
                .text_size(px(12.))
                .text_color(p.foreground_subtle)
                .child(version),
        )
        .child(div().flex_1())
        .child(reload_btn)
        .child(details_btn)
        .child(menu(view, &pane.menu, cx))
        .child(stop_btn)
}
