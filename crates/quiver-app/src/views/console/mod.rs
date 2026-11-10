//! The console dock: the daemon's log and a prompt, under the page. One entity for the whole session
//! (the shell owns it) so what was printed survives closing the dock. The decisions are
//! `quiver_state::console::ConsoleController`'s; `effects` carries out what it asks for.

mod effects;
mod parts;

use std::collections::HashMap;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, ScrollHandle, Subscription, Task, Window};
use quiver_state::console::{ConsoleController, ConsoleEffect, TimerKind};
use quiver_state::domain::ConnectionStatus;
use quiver_ui::{ActiveQuiver, InputState};

use crate::bridge::observe_store;
use crate::model::AppModel;

pub const DOCK_HEIGHT: f32 = 264.;

pub struct ConsoleDock {
    pub(super) app: Entity<AppModel>,
    pub(super) controller: ConsoleController,
    pub(super) input: Entity<InputState>,
    pub(super) scroll: ScrollHandle,
    pub(super) tail_seen: u64,
    pub(super) entries_seen: usize,
    exec_manager: std::sync::Arc<quiver_core_client::console::ConsoleExecManager>,
    pub(super) exec_tasks: HashMap<u64, Task<()>>,
    pub(super) streams: HashMap<u64, (quiver_state::transport::ws_manager::Subscription, Task<()>)>,
    pub(super) timers: Vec<(TimerKind, Task<()>)>,
    _subscriptions: Vec<Subscription>,
}

impl ConsoleDock {
    pub fn new(app: Entity<AppModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.status.store());
        observe_store(cx, services.connections.store());
        let placeholder = app
            .read(cx)
            .t(quiver_state::i18n::en::console::PROMPT_PLACEHOLDER);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let subscriptions = vec![
            cx.subscribe_in(&input, window, Self::input_event),
            cx.observe(&app, |this, _, cx| this.model_changed(cx)),
        ];
        let mut dock = Self {
            app,
            controller: ConsoleController::default(),
            input,
            scroll: ScrollHandle::new(),
            tail_seen: 0,
            entries_seen: 0,
            exec_manager: services.console.clone(),
            exec_tasks: HashMap::new(),
            streams: HashMap::new(),
            timers: Vec::new(),
            _subscriptions: subscriptions,
        };
        dock.sync_connection(cx);
        if dock.app.read(cx).demo {
            dock.controller.state.support = quiver_state::console::Support::Supported;
            dock.controller
                .state
                .push(crate::demo::screens::console_entries());
            if std::env::var_os("QUIVER_DEMO_CONSOLE").is_some() {
                dock.app.update(cx, |m, cx| m.set_console_open(true, cx));
            }
        }
        dock
    }

    /// Pushes the app's idea of "open" and of the connection into the controller.
    fn model_changed(&mut self, cx: &mut Context<Self>) {
        let open = self.app.read(cx).console_open;
        if open != self.controller.state.open {
            let fx = self.controller.set_open(open);
            self.run_effects(fx, cx);
        }
        self.sync_connection(cx);
        cx.notify();
    }

    pub(super) fn sync_connection(&mut self, cx: &mut Context<Self>) {
        let (id, ready) = {
            let services = &self.app.read(cx).services;
            (
                services.connections.snapshot().active_id,
                services.status.status() == ConnectionStatus::Ready,
            )
        };
        let fx = self.controller.sync(&id, ready);
        self.run_effects(fx, cx);
    }

    /// Escape and the header's close button.
    pub(super) fn close(&mut self, cx: &mut Context<Self>) {
        self.app.update(cx, |m, cx| m.set_console_open(false, cx));
    }
}

impl Drop for ConsoleDock {
    fn drop(&mut self) {
        let fx = self.controller.dispose();
        for effect in fx {
            if let ConsoleEffect::CancelExec { run } = effect {
                self.exec_manager.cancel(&format!("native-console-{run}"));
            }
        }
    }
}

impl Render for ConsoleDock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.controller.state.open {
            return div().into_any_element();
        }
        let p = cx.quiver().palette;
        self.follow_tail();
        div()
            .id("console")
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .h(px(DOCK_HEIGHT))
            .border_t_1()
            .border_color(p.border)
            .bg(p.background)
            .on_key_down(cx.listener(Self::key_down))
            .child(parts::header(self, cx))
            .child(parts::log(self, cx))
            .child(parts::prompt(self, window, cx))
            .child(crate::views::screen_probe("console"))
            .into_any_element()
    }
}
