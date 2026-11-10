//! Carries out what the console controller asks for: the two reads, the log socket, the batch and
//! backoff timers, and the commands. Each answer goes back through the controller's feedback methods,
//! which may ask for more.

use std::time::Duration;

use gpui::Context;
use quiver_core_client::bridge::FrameSink;
use quiver_core_client::console::EXEC_DEADLINE;
use quiver_state::console::{parse_commands, parse_versions, ConsoleEffect};
use quiver_state::core_api::CoreApiExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use super::ConsoleDock;
use crate::bridge::spawn_action;

struct Lines(UnboundedSender<String>);

impl FrameSink for Lines {
    fn send(
        &self,
        frame: String,
    ) {
        // The dock going away drops the receiver; a frame for nobody is dropped (see FrameSink).
        let _ = self.0.send(frame);
    }
}

fn exec_id(run: u64) -> String {
    format!("native-console-{run}")
}

impl ConsoleDock {
    pub(super) fn run_effects(
        &mut self,
        effects: Vec<ConsoleEffect>,
        cx: &mut Context<Self>,
    ) {
        for effect in effects {
            self.run_effect(effect, cx);
        }
        cx.notify();
    }

    fn run_effect(
        &mut self,
        effect: ConsoleEffect,
        cx: &mut Context<Self>,
    ) {
        let (services, handle) = {
            let m = self.app.read(cx);
            (m.services.clone(), m.handle().clone())
        };
        match effect {
            ConsoleEffect::FetchVersions { epoch } => {
                let api = services.api.clone();
                spawn_action(
                    cx,
                    &handle,
                    async move {
                        api.get_json("/versions")
                            .await
                            .ok()
                            .and_then(|v| parse_versions(&v))
                    },
                    move |this: &mut Self, versions, cx| {
                        let fx = this.controller.on_versions(epoch, versions);
                        this.run_effects(fx, cx);
                    },
                );
            }
            ConsoleEffect::FetchCommands { epoch, for_help } => {
                let api = services.api.clone();
                spawn_action(
                    cx,
                    &handle,
                    async move {
                        api.get_json("/v0/console/commands")
                            .await
                            .ok()
                            .map(|v| parse_commands(&v))
                    },
                    move |this: &mut Self, commands, cx| {
                        this.controller.on_commands(epoch, commands, for_help);
                        cx.notify();
                    },
                );
            }
            ConsoleEffect::OpenStream { path, attempt } => {
                let (tx, mut rx) = unbounded_channel::<Option<String>>();
                let _guard = handle.enter();
                let subscription = services.ws.subscribe(&path, move |value| {
                    let message =
                        if quiver_state::transport::ws_manager::is_reconnect_sentinel(value) {
                            None
                        } else {
                            Some(value.to_string())
                        };
                    let _ = tx.send(message);
                });
                let task = cx.spawn(async move |this, cx| {
                    this.update(cx, |this, cx| {
                        this.controller.stream_opened(attempt);
                        cx.notify();
                    })
                    .ok();
                    while let Some(message) = rx.recv().await {
                        let alive = this.update(cx, |this, cx| {
                            let fx = match &message {
                                Some(text) => this.controller.stream_message(attempt, text),
                                None => this.controller.stream_closed(attempt),
                            };
                            this.run_effects(fx, cx);
                        });
                        if alive.is_err() {
                            break;
                        }
                    }
                });
                self.streams.insert(attempt, (subscription, task));
            }
            ConsoleEffect::CloseStream { attempt } => {
                self.streams.remove(&attempt);
            }
            ConsoleEffect::StartTimer { kind, ms } => {
                let task = cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(ms))
                        .await;
                    this.update(cx, |this, cx| {
                        this.timers.retain(|(k, _)| *k != kind);
                        let fx = this.controller.timer_elapsed(kind);
                        this.run_effects(fx, cx);
                    })
                    .ok();
                });
                self.timers.retain(|(k, _)| *k != kind);
                self.timers.push((kind, task));
            }
            ConsoleEffect::CancelTimer { kind } => self.timers.retain(|(k, _)| *k != kind),
            ConsoleEffect::StartExec { run, line } => {
                let (tx, mut rx) = unbounded_channel::<String>();
                let console = services.console.clone();
                let manager = services.manager.clone();
                let started = handle.spawn(async move {
                    let transport = manager.transport().await;
                    console.start(exec_id(run), transport, line, Lines(tx), EXEC_DEADLINE);
                });
                let task = cx.spawn(async move |this, cx| {
                    let _ = started.await;
                    while let Some(frame) = rx.recv().await {
                        let alive = this.update(cx, |this, cx| {
                            this.controller.exec_frame(run, &frame);
                            cx.notify();
                        });
                        if alive.is_err() {
                            return;
                        }
                    }
                    this.update(cx, |this, cx| {
                        this.controller.exec_closed(run);
                        this.exec_tasks.remove(&run);
                        cx.notify();
                    })
                    .ok();
                });
                self.exec_tasks.insert(run, task);
            }
            ConsoleEffect::CancelExec { run } => {
                services.console.cancel(&exec_id(run));
                self.exec_tasks.remove(&run);
            }
        }
    }
}

impl ConsoleDock {
    /// Ctrl+C or the Cancel button.
    pub(super) fn cancel(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let note = self
            .app
            .read(cx)
            .t(quiver_state::i18n::en::screens::CONSOLE_CANCEL);
        let fx = self.controller.cancel_running(&note);
        self.run_effects(fx, cx);
    }

    /// Runs the line in the prompt.
    pub(super) fn run_line(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let line = self.input.read(cx).value().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let fx = self.controller.submit(&line, now);
        self.input.update(cx, |i, cx| i.set_value("", window, cx));
        self.run_effects(fx, cx);
    }
}
