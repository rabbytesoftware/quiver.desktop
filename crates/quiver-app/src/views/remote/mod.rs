//! Remote Control (`RemoteAfter`): the saved connections, one row each, and the Add / Rename / Remove
//! dialogs. The flows and their outcomes are `quiver_state::remote`'s; this draws them.

mod actions;
mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, Window};
use quiver_state::domain::ConnectionStatus;
use quiver_state::remote::{connection_rows, ToastStack};
use quiver_ui::{ActiveQuiver, ScrollArea};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::screen_probe;

pub struct RemoteView {
    pub(super) app: Entity<AppModel>,
    pub(super) toasts: ToastStack,
    /// An Add is in flight: the dialog's submit is disabled meanwhile.
    pub(super) add_busy: bool,
}

impl RemoteView {
    pub fn new(app: Entity<AppModel>, cx: &mut Context<Self>) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.connections.store());
        observe_store(cx, services.status.store());
        Self {
            app,
            toasts: ToastStack::default(),
            add_busy: false,
        }
    }
}

impl Render for RemoteView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let (state, status) = {
            let services = &self.app.read(cx).services;
            (services.connections.snapshot(), services.status.status())
        };
        // The list arrives from a one-shot read plus a live event; until either landed it is empty. Local
        // is always present once it has, so an empty list is "not loaded yet".
        let loading = state.connections.is_empty() && status != ConnectionStatus::Disconnected;
        let rows = connection_rows(&state.connections, &state.active_id, status);
        let body = if loading {
            parts::loading(cx).into_any_element()
        } else if rows.is_empty() {
            parts::empty(self, cx).into_any_element()
        } else {
            parts::rows(self, &rows, cx).into_any_element()
        };
        div()
            .id("remote")
            .relative()
            .size_full()
            .bg(p.background)
            .child(
                ScrollArea::new("remote.scroll")
                    .fade_color(p.background)
                    .child(
                        div()
                            .px(px(40.))
                            .pl(px(24.))
                            .pt(px(24.))
                            .pb(px(32.))
                            .child(parts::header(self, cx))
                            .child(div().mt(px(4.)).child(body)),
                    ),
            )
            .child(parts::toasts(self, cx))
            .child(screen_probe("remote"))
    }
}
