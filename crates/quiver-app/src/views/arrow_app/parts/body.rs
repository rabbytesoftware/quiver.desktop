//! Under the header: the app's page. The page is a webview embedded in the main window that sits over
//! this body; the body reports its rectangle to the supervisor on every paint so the page follows
//! layout and resizes. While the app is starting, or after it closed, the body says so instead, and
//! carries the notices (eviction, a page that would not open).

use gpui::prelude::*;
use gpui::{canvas, div, px, Context, FontWeight, IntoElement};
use quiver_arrow_host::PageBounds;
use quiver_state::arrow_app::hosting::{pane_body, PaneBody};
use quiver_state::i18n::en::{arrowapp, screens};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Spinner};

use crate::views::arrow_app::ArrowAppView;

pub fn body(view: &mut ArrowAppView, cx: &mut Context<ArrowAppView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let (entry, windows, demo) = {
        let m = view.app.read(cx);
        (
            m.services.arrows.entry(&view.namespace),
            m.services.arrow_windows.clone(),
            m.demo,
        )
    };
    let ready = entry
        .as_ref()
        .and_then(|a| a.active_run.as_ref())
        .and_then(|r| r.surface.as_ref())
        .is_some_and(|s| s.ready);
    let open = demo || windows.open_apps().contains(&view.namespace);
    let state = pane_body(ready, open);
    let name = entry.map_or_else(|| view.namespace.clone(), |a| a.name);
    let m = view.app.read(cx);
    let title = match state {
        PaneBody::Starting => m.t(arrowapp::STARTING),
        // The page covers the body; this only shows while it loads (and in the demo, which has no page).
        PaneBody::Open => name,
        PaneBody::Closed => m.t(screens::ARROWAPP_WINDOW_CLOSED),
    };
    let (reporter, namespace) = (windows.clone(), view.namespace.clone());
    div()
        .relative()
        .child(
            canvas(
                move |bounds, _, _| {
                    reporter.set_bounds(
                        &namespace,
                        PageBounds {
                            x: f32::from(bounds.origin.x),
                            y: f32::from(bounds.origin.y),
                            width: f32::from(bounds.size.width),
                            height: f32::from(bounds.size.height),
                        },
                    );
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .text_color(p.muted_foreground)
        .when(state == PaneBody::Starting, |d| {
            d.child(Spinner::new("arrow-app-body-spinner").size(18.))
        })
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.foreground)
                .child(title),
        )
        .children(view.notice.clone().map(|n| {
            div()
                .relative()
                .mt(px(8.))
                .text_size(px(12.5))
                .text_color(p.warning)
                .child(n.clone())
                .child(probe("arrow-app.notice", Kind::Text, n, true, ""))
        }))
        .child(probe(
            "arrow-app.body",
            Kind::Other,
            "body",
            true,
            match state {
                PaneBody::Starting => "starting",
                PaneBody::Open => "open",
                PaneBody::Closed => "closed",
            },
        ))
}
