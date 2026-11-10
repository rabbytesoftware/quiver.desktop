//! Under the header: where the app's page is. The page itself is the helper window's; this says whether
//! it is starting, open or closed, and carries the notices (eviction, a window that would not open).

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::arrow_app::hosting::{pane_body, PaneBody};
use quiver_state::i18n::en::{arrowapp, screens};
use quiver_state::i18n::Params;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant, Spinner};

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
    let (title, text) = match state {
        PaneBody::Starting => (m.t(arrowapp::STARTING), None),
        PaneBody::Open => (
            m.t(screens::ARROWAPP_WINDOW_TITLE),
            Some(m.translator.t_with(
                screens::ARROWAPP_WINDOW_BODY,
                &Params::new().with("name", name.as_str()),
            )),
        ),
        PaneBody::Closed => (m.t(screens::ARROWAPP_WINDOW_CLOSED), None),
    };
    let open_label = m.t(screens::ARROWAPP_OPEN_WINDOW);
    let this = cx.entity();
    div()
        .relative()
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
        .children(text.map(|t| div().max_w(px(420.)).text_size(px(13.)).child(t)))
        .when(state == PaneBody::Closed, |d| {
            d.child(
                div().relative().child(
                    Button::new("arrow-app-open-window")
                        .variant(ButtonVariant::Outline)
                        .label(open_label.clone())
                        .on_click(move |_, _, cx| this.update(cx, |v, cx| v.show_window(cx)))
                        .child(probe(
                            "arrow-app.open-window",
                            Kind::Button,
                            open_label,
                            true,
                            "",
                        )),
                ),
            )
        })
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
