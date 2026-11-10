//! `quiver_e2e::server::Driver` over the live GPUI window.

use super::commands::{Command, Reply};
use super::{logs, screenshot, StateSnapshot};
use async_trait::async_trait;
use quiver_e2e::protocol::{Node, Tree};
use quiver_e2e::server::Driver;
use serde_json::Value;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

pub struct GpuiDriver {
    commands: UnboundedSender<Command>,
    state: StateSnapshot,
}

impl GpuiDriver {
    pub fn new(
        commands: UnboundedSender<Command>,
        state: StateSnapshot,
    ) -> Self {
        Self { commands, state }
    }

    async fn ask(
        &self,
        build: impl FnOnce(Reply) -> Command,
    ) -> Result<(), String> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(build(reply))
            .map_err(|_| "the app is shutting down".to_string())?;
        answer
            .await
            .map_err(|_| "the app dropped the command".to_string())?
    }
}

#[async_trait]
impl Driver for GpuiDriver {
    async fn tree(&self) -> Tree {
        let nodes = quiver_ui::inspect::snapshot()
            .into_iter()
            .map(|n| Node {
                id: n.id.to_string(),
                kind: n.kind.as_str().to_string(),
                label: n.label.to_string(),
                enabled: n.enabled,
                state: n.state.to_string(),
                x: n.x,
                y: n.y,
                w: n.w,
                h: n.h,
            })
            .collect();
        Tree {
            frame: quiver_ui::inspect::frame(),
            nodes,
        }
    }

    async fn click(
        &self,
        id: &str,
        double: bool,
    ) -> Result<(), String> {
        let id = id.to_string();
        self.ask(|reply| Command::Click { id, double, reply }).await
    }

    async fn hover(
        &self,
        id: &str,
    ) -> Result<(), String> {
        let id = id.to_string();
        self.ask(|reply| Command::Hover { id, reply }).await
    }

    async fn type_text(
        &self,
        id: Option<&str>,
        text: &str,
    ) -> Result<(), String> {
        let (id, text) = (id.map(str::to_string), text.to_string());
        self.ask(|reply| Command::Type { id, text, reply }).await
    }

    async fn key(
        &self,
        keys: &str,
    ) -> Result<(), String> {
        let keys = keys.to_string();
        self.ask(|reply| Command::Key { keys, reply }).await
    }

    async fn scroll(
        &self,
        id: &str,
        dx: f32,
        dy: f32,
    ) -> Result<(), String> {
        let id = id.to_string();
        self.ask(|reply| Command::Scroll { id, dx, dy, reply })
            .await
    }

    async fn state(&self) -> Value {
        (self.state)()
    }

    async fn logs(
        &self,
        n: usize,
    ) -> Vec<String> {
        logs::recent(n)
    }

    async fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        tokio::task::spawn_blocking(screenshot::capture_window)
            .await
            .map_err(|e| e.to_string())?
    }

    async fn resize(
        &self,
        width: f32,
        height: f32,
    ) -> Result<(), String> {
        self.ask(|reply| Command::Resize {
            width,
            height,
            reply,
        })
        .await
    }
}
