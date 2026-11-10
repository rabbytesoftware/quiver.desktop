//! What the arrow page shows while the arrow has an interface (`ArrowAppAfter`). The header the Tauri app
//! drew over the iframe -- name, status, Reload, Details, the actions menu and Stop -- and, under it, the
//! app's page: a webview embedded in the main window over the pane body (docs/decision-log.md). Details
//! opens the arrow's own page in a dialog over the app, as in Tauri; the page stays alive but steps aside
//! while any dialog or menu is open, because a native view paints above GPUI. It drives
//! `services.arrow_windows`: showing the page when the surface answers, hiding it when the pane is left,
//! closing it on Stop, and telling the user when the 4-app cap closed another.

mod parts;

use gpui::prelude::*;
use gpui::{div, Context, Entity, Render, Task, Window};
use quiver_state::arrow_app::hosting::{host_step, HostStep};
use quiver_state::arrow_app::surface_namespaces;
use quiver_state::i18n::en::screens;
use quiver_state::i18n::Params;
use quiver_state::namespace::bare_namespace;
use quiver_ui::ActiveQuiver;
use quiver_ui::Dialog;

use crate::bridge::spawn_action;
use crate::model::AppModel;
use crate::services::WindowEvent;
use crate::views::arrow_details::ArrowDetailsView;
use crate::views::screen_probe;

pub struct ArrowAppView {
    pub(super) app: Entity<AppModel>,
    pub(super) namespace: String,
    /// The last thing worth telling the user (an eviction, a window that would not open).
    pub(super) notice: Option<String>,
    details: Entity<ArrowDetailsView>,
    pub(super) details_open: bool,
    services: std::sync::Arc<crate::services::AppServices>,
    _watchers: Vec<Task<()>>,
}

impl ArrowAppView {
    pub fn new(
        app: Entity<AppModel>,
        namespace: String,
        details: Entity<ArrowDetailsView>,
        cx: &mut Context<Self>,
    ) -> Self {
        let services = app.read(cx).services.clone();
        // The route carries the bare namespace; the store, the windows and the daemon calls are keyed by the
        // catalog identity (`ns@selector`).
        let namespace = services
            .arrows
            .entries()
            .into_iter()
            .find(|e| bare_namespace(&e.namespace) == namespace)
            .map_or(namespace, |e| e.namespace);
        let mut revisions = services.arrows.store().subscribe();
        let store_task = cx.spawn(async move |this, cx| {
            while revisions.changed().await.is_ok() {
                if this.update(cx, |this, cx| this.sync(cx)).is_err() {
                    break;
                }
            }
        });
        let mut events = services.arrow_windows.subscribe();
        let window_task = cx.spawn(async move |this, cx| {
            while let Some(event) = events.recv().await {
                if this
                    .update(cx, |this, cx| this.window_event(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut view = Self {
            app,
            namespace,
            notice: None,
            details,
            details_open: false,
            services,
            _watchers: vec![store_task, window_task],
        };
        view.sync(cx);
        view
    }

    fn window_event(&mut self, event: WindowEvent, cx: &mut Context<Self>) {
        if let WindowEvent::Evicted { namespace } = event {
            let m = self.app.read(cx);
            let name = quiver_state::arrow_app::hosting::eviction_name(
                &namespace,
                m.services
                    .arrows
                    .entry(&namespace)
                    .map(|a| a.name)
                    .as_deref(),
            );
            self.notice = Some(m.translator.t_with(
                screens::ARROWAPP_EVICTED,
                &Params::new().with("name", name).with("max", 4u32),
            ));
        }
        cx.notify();
    }

    /// Brings the window in step with the arrow: prune what has no surface, show this app once its page
    /// answers, and fall back to the arrow's page when the surface is gone.
    pub(super) fn sync(&mut self, cx: &mut Context<Self>) {
        let (demo, services) = {
            let m = self.app.read(cx);
            (m.demo, m.services.clone())
        };
        let entry = services.arrows.entry(&self.namespace);
        if demo {
            cx.notify();
            return;
        }
        let live = surface_namespaces(&services.arrows.entries());
        services
            .arrow_windows
            .prune(&live.iter().cloned().collect());
        let surface = entry
            .as_ref()
            .and_then(|a| a.active_run.as_ref())
            .and_then(|r| r.surface.as_ref());
        if surface.is_none() {
            cx.notify();
            return;
        }
        let ready = surface.is_some_and(|s| s.ready);
        let visible = services.arrow_windows.visible();
        match host_step(Some(&self.namespace), ready, visible.as_deref()) {
            HostStep::Show => self.show_window(cx),
            HostStep::Hide => services.arrow_windows.hide(),
            HostStep::Keep => {}
        }
        cx.notify();
    }

    /// The arrow's own page over the app (Tauri's Details dialog). The app's page steps aside meanwhile.
    pub(super) fn open_details(&mut self, cx: &mut Context<Self>) {
        self.details_open = true;
        self.services.arrow_windows.set_covered(true);
        cx.notify();
    }

    pub(super) fn close_details(&mut self, cx: &mut Context<Self>) {
        self.details_open = false;
        self.services.arrow_windows.set_covered(false);
        cx.notify();
    }

    /// Reload remounts the page; when the page is gone (evicted, or it never opened) it opens it again.
    pub(super) fn reload(&mut self, cx: &mut Context<Self>) {
        if self
            .services
            .arrow_windows
            .open_apps()
            .contains(&self.namespace)
        {
            self.services.arrow_windows.reload(&self.namespace);
            return;
        }
        self.show_window(cx);
    }

    pub(super) fn show_window(&mut self, cx: &mut Context<Self>) {
        let (windows, handle, title) = {
            let m = self.app.read(cx);
            let title = m
                .services
                .arrows
                .entry(&self.namespace)
                .map_or_else(|| self.namespace.clone(), |a| a.name);
            (m.services.arrow_windows.clone(), m.handle().clone(), title)
        };
        let namespace = self.namespace.clone();
        spawn_action(
            cx,
            &handle,
            async move { windows.show(&namespace, &title).await },
            |this: &mut Self, result, cx| {
                // Only a failure writes the notice: a success must not wipe the eviction notice the same
                // `show` just emitted (the event lands before this callback runs).
                if let Err(e) = result {
                    this.notice = Some(this.app.read(cx).translator.t_with(
                        screens::ARROWAPP_FAILED,
                        &Params::new().with("reason", e.to_string()),
                    ));
                }
                cx.notify();
            },
        );
    }
}

impl Drop for ArrowAppView {
    /// Leaving the pane hides the window; it stays alive so the app keeps its state. Only when this
    /// pane's app is the visible one: the next pane may already have shown its own.
    fn drop(&mut self) {
        self.services.arrow_windows.set_covered(false);
        if self.services.arrow_windows.visible().as_deref() == Some(self.namespace.as_str()) {
            self.services.arrow_windows.hide();
        }
    }
}

impl Render for ArrowAppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let this = cx.entity();
        let height = f32::from(window.viewport_size().height) * 0.9;
        div()
            .id("arrow-app")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .child(parts::header(self, cx))
            .child(parts::body(self, cx))
            .child(
                Dialog::new("arrow-app-details-dialog")
                    .open(self.details_open)
                    .width(1024.)
                    .show_close(false)
                    .on_dismiss(move |_, cx| this.update(cx, |v, cx| v.close_details(cx)))
                    .child(
                        div()
                            .w_full()
                            .h(gpui::px(height))
                            .overflow_hidden()
                            .child(self.details.clone()),
                    ),
            )
            .child(screen_probe("arrow-app"))
    }
}
