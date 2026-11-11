//! The left rail (`SidebarAfter`): back/forward and the build indicator on top, the search field,
//! the Home/Remote/Settings tabs, the arrow list and the resize handle.

mod parts;

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, Render, Subscription, Task, UniformListScrollHandle, Window,
};
use quiver_state::search::{
    commit_target, needs_namespace_lookup, resolve_namespace, submit_action, CommitTarget,
    NamespaceTarget, SubmitAction, TargetKind,
};
use quiver_state::shell::SidebarSide;
use quiver_ui::{ActiveQuiver, InputEvent, InputState};

use crate::bridge::{observe_store, spawn_action};
use crate::model::{AppModel, Route};

pub struct Sidebar {
    pub(super) app: Entity<AppModel>,
    pub(super) search: Entity<InputState>,
    pub(super) list_scroll: UniformListScrollHandle,
    /// The in-app row menu, shown only when the OS's own could not be (see `parts::context_menu`).
    pub(super) context: Option<parts::RowMenu>,
    /// A native row menu is up.
    pub(super) menu_busy: bool,
    debounce: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Sidebar {
    pub fn new(
        app: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search Arrows"));
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        observe_store(cx, services.status.store());
        observe_store(cx, services.connections.store());
        let core_versions = app.read(cx).core_versions.clone();
        observe_store(cx, &core_versions);
        let subscriptions = vec![
            cx.observe_in(&app, window, Self::route_changed),
            cx.subscribe_in(&search, window, Self::search_event),
        ];
        Self {
            app,
            search,
            list_scroll: UniformListScrollHandle::new(),
            context: None,
            menu_busy: false,
            debounce: None,
            _subscriptions: subscriptions,
        }
    }

    /// The URL is the source of truth for a results page: keep the field in step with it.
    fn route_changed(
        &mut self,
        app: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Route::Search { query } = app.read(cx).route().clone() {
            if self.search.read(cx).value().as_ref() != query {
                self.search
                    .update(cx, |input, cx| input.set_value(query, window, cx));
            }
        }
        cx.notify();
    }

    fn showing_results(
        &self,
        cx: &App,
    ) -> bool {
        matches!(self.app.read(cx).route(), Route::Search { .. })
    }

    fn commit(
        &mut self,
        target: CommitTarget,
        cx: &mut Context<Self>,
    ) {
        let (route, replace) = match target {
            CommitTarget::Home { replace } => (Route::Home, replace),
            CommitTarget::Results { query, replace } => (Route::Search { query }, replace),
        };
        self.app.update(cx, |app, cx| {
            if replace {
                app.replace(route, cx);
            } else {
                app.navigate(route, cx);
            }
        });
    }

    /// What Enter does once the lookup (if any) has answered.
    fn submit(
        &mut self,
        text: &str,
        resolved: Option<NamespaceTarget>,
        cx: &mut Context<Self>,
    ) {
        match submit_action(text, resolved, self.showing_results(cx)) {
            SubmitAction::GoTo(target) => {
                let route = match target.kind {
                    TargetKind::Arrow => Route::Arrow {
                        namespace: target.namespace,
                    },
                    TargetKind::Collection => Route::Collection {
                        id: target.namespace,
                    },
                };
                self.app.update(cx, |app, cx| app.navigate(route, cx));
            }
            SubmitAction::Search {
                request_submit,
                then,
            } => {
                if let Some(query) = request_submit {
                    crate::views::search::session::session(cx).request_submit(&query);
                }
                self.commit(then, cx);
            }
        }
        cx.notify();
    }

    fn search_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = input.read(cx).value().to_string();
        match event {
            InputEvent::Change => {
                if text.is_empty() && !self.showing_results(cx) {
                    return;
                }
                let target = commit_target(&text, self.showing_results(cx));
                self.debounce = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(
                            quiver_state::search::LOCAL_DEBOUNCE_MS,
                        ))
                        .await;
                    this.update(cx, |this, cx| this.commit(target, cx)).ok();
                }));
            }
            InputEvent::PressEnter { .. } => {
                self.debounce = None;
                if !needs_namespace_lookup(&text) {
                    self.submit(&text, None, cx);
                    return;
                }
                // Enter on something namespace-shaped goes there if it exists. A daemon that does not
                // answer reads as "nothing there": the draft is searched for instead.
                let (api, handle) = {
                    let app = self.app.read(cx);
                    (app.services.api.clone(), app.handle().clone())
                };
                let draft = text.clone();
                spawn_action(
                    cx,
                    &handle,
                    async move { resolve_namespace(api.as_ref(), &draft).await.ok().flatten() },
                    move |this: &mut Sidebar, resolved, cx| this.submit(&text, resolved, cx),
                );
            }
            _ => {}
        }
        cx.notify();
    }
}

impl Render for Sidebar {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let (width, side) = {
            let shell = self.app.read(cx).shell;
            (shell.sidebar_width as f32, shell.sidebar_side)
        };
        div()
            .id("sidebar")
            .relative()
            .flex_none()
            .w(px(width))
            .h_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(p.sidebar)
            .text_color(p.sidebar_foreground)
            .when(side == SidebarSide::Left, |d| d.border_r_1())
            .when(side == SidebarSide::Right, |d| d.border_l_1())
            .border_color(p.sidebar_border)
            .child(parts::top_row(self, cx))
            .child(parts::search_and_tabs(self, cx))
            .child(parts::arrow_list(self, window, cx))
            .child(parts::resize_handle(self, cx))
            .children(parts::context_menu(self, cx))
    }
}
