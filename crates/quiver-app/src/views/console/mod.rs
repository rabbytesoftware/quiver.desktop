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

pub const DOCK_HEIGHT: f32 = 420.;

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
    pub fn new(
        app: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
    fn model_changed(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let open = self.app.read(cx).console_open;
        if open != self.controller.state.open {
            let fx = self.controller.set_open(open);
            self.run_effects(fx, cx);
        }
        self.sync_connection(cx);
        cx.notify();
    }

    pub(super) fn sync_connection(
        &mut self,
        cx: &mut Context<Self>,
    ) {
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
    pub(super) fn close(
        &mut self,
        cx: &mut Context<Self>,
    ) {
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
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if !self.controller.state.open {
            return div().into_any_element();
        }
        let p = cx.quiver().palette;
        let tokens = quiver_theme::Tokens::for_mode(cx.quiver().mode);
        let colors = parts::Colors::of(&tokens);
        self.follow_tail();
        let height = (window.viewport_size().height * 0.7).min(px(DOCK_HEIGHT));
        div()
            .id("console")
            .absolute()
            // Drawn over the page, so without this a wheel over the console also reached the scroll area behind it.
            .occlude()
            .top_0()
            .left_0()
            .right_0()
            .h(height)
            .flex()
            .flex_col()
            .border_b_1()
            .border_color(p.border)
            .bg(colors.surface)
            .text_color(colors.foreground)
            .on_key_down(cx.listener(Self::key_down))
            .child(parts::log(self, cx))
            .child(parts::prompt(self, window, cx))
            .child(crate::views::screen_probe("console"))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;
    use gpui::{point, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase};

    /// A tall scrolling page with the console drawn over its top (as in the shell).
    struct Harness {
        dock: Entity<ConsoleDock>,
        page: ScrollHandle,
    }

    impl Render for Harness {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .child(
                    div()
                        .id("page")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.page)
                        .child(div().h(px(6000.))),
                )
                .child(self.dock.clone())
        }
    }

    fn wheel(y: f32) -> ScrollWheelEvent {
        ScrollWheelEvent {
            position: point(px(300.), px(y)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        }
    }

    /// The console is drawn over the page, so a wheel over it reached both and the page behind scrolled
    /// with the log. The console must take the wheel for itself and leave the page where it is.
    #[gpui::test]
    fn the_wheel_over_the_console_does_not_scroll_the_page_behind(cx: &mut TestAppContext) {
        let app = demo_app(cx);
        cx.update(|cx| quiver_ui::install(cx, quiver_theme::Mode::Light));
        let page = ScrollHandle::new();
        let (view, cx) = cx.add_window_view(|window, cx| Harness {
            dock: cx.new(|cx| ConsoleDock::new(app.clone(), window, cx)),
            page: page.clone(),
        });
        // Open it once the dock exists: it learns the app's "open" from the change notification.
        app.update(cx, |m, cx| m.set_console_open(true, cx));
        cx.run_until_parked();
        let dock = view.read_with(cx, |h, _| h.dock.clone());
        assert!(
            dock.read_with(cx, |d, _| d.controller.state.open),
            "the console never opened, so this test would prove nothing"
        );
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();

        cx.simulate_event(wheel(20.));
        assert_eq!(
            page.offset().y,
            px(0.),
            "the page moved under a wheel over the console"
        );

        // Control: the same wheel below the console does scroll the page, so the check above can fail.
        cx.simulate_event(wheel(700.));
        assert!(
            page.offset().y < px(0.),
            "the page does not scroll at all in this harness"
        );
    }
}
